//! Phase 2 (mapped performance foundation) tests: `.cfr` v3 precomputed
//! sections round-trip, pre-v3 cache fallback, oslots slot-identity, canonical
//! `F.s` ordering, the binary-search edge-value index, and view-cache hit
//! consistency. Fixtures are tiny corpora compiled to a temp `.cfr`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use context_fabric_core::compiled::{MappedCompiledCorpus, MappedNodeValue, compile_features};
use context_fabric_core::{Corpus, FeatureValue, precompute};

fn mini_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini_corpus")
}

/// Copies the mini corpus into a temp dir so individual tests can add features.
fn copy_mini_corpus(dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    for entry in fs::read_dir(mini_corpus_dir()).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("tf") {
            fs::copy(&path, dest.join(path.file_name().unwrap())).unwrap();
        }
    }
}

/// Builds a 3-level (book > chapter > verse > word) corpus that fully exercises
/// the `CFRSECT1` sections data (sec1, sec2, seq_from_node, node_from_seq).
fn write_three_level_corpus(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    let files = [
        (
            "otype.tf",
            "@node\n@valueType=str\n\n\
             word\nword\nword\nword\nword\nword\n\
             7\tverse\n8\tverse\n9\tchapter\n10\tbook\n",
        ),
        (
            "oslots.tf",
            "@edge\n@valueType=int\n\n7\t1-3\n8\t4-6\n9\t1-6\n10\t1-6\n",
        ),
        ("word.tf", "@node\n@valueType=str\n\na\nb\nc\nd\ne\nf\n"),
        ("bookname.tf", "@node\n@valueType=str\n\n10\tGenesis\n"),
        ("chapternum.tf", "@node\n@valueType=int\n\n9\t1\n"),
        ("versenum.tf", "@node\n@valueType=int\n\n7\t1\n8\t2\n"),
        (
            "otext.tf",
            "@config\n@fmt:text-orig-full={word}\n\
             @sectionTypes=book,chapter,verse\n\
             @sectionFeatures=bookname,chapternum,versenum\n\
             @structureTypes=\n@structureFeatures=\n",
        ),
    ];
    for (name, contents) in files {
        fs::write(dir.join(name), contents).unwrap();
    }
}

#[test]
fn v3_sections_round_trip_matches_precompute_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("three_level");
    write_three_level_corpus(&source);
    let cache = temp.path().join("three_level.cfr");
    compile_features(&source, &cache, &[]).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache).unwrap();
    let corpus = Corpus::load(&source).unwrap();

    // Recompute the reference precomputes exactly as the writer does.
    let slot_sets: std::collections::BTreeMap<u32, Vec<u32>> =
        corpus.oslots_items().into_iter().collect();
    let rank = corpus.rank();
    let max_slot = corpus.max_slot();
    let max_node = corpus.max_node();
    let lev_up = precompute::lev_up(&slot_sets, &rank, max_slot, max_node);
    let lev_down = precompute::lev_down(&lev_up, &rank, max_slot, max_node);
    let boundary = precompute::boundary(&slot_sets, &rank, max_slot);
    let sections = precompute::sections(&corpus).expect("3-level corpus has sections");

    // v3 sections present in metadata.
    let meta = mapped.metadata();
    assert!(meta.v3_start.is_some(), "v3 region must be present");
    assert!(meta.lev_up_start.is_some());
    assert!(meta.lev_down_start.is_some());
    assert!(meta.boundary_first_start.is_some());
    assert!(meta.boundary_last_start.is_some());
    assert!(meta.sections_start.is_some());

    // levUp rows.
    for node in 1..=max_node {
        let expected = &lev_up[(node - 1) as usize];
        assert_eq!(
            mapped.lev_up_row(node).unwrap().as_deref(),
            Some(expected.as_slice()),
            "lev_up_row mismatch for node {node}"
        );
    }

    // levDown rows (stored for all nodes; non-slot rows match precompute).
    for node in (max_slot + 1)..=max_node {
        let expected = &lev_down[(node - max_slot - 1) as usize];
        assert_eq!(
            mapped.lev_down_row(node).unwrap().as_deref(),
            Some(expected.as_slice()),
            "lev_down_row mismatch for node {node}"
        );
    }
    // Slots have empty levDown rows.
    for slot in 1..=max_slot {
        assert_eq!(mapped.lev_down_row(slot).unwrap(), Some(Vec::new()));
    }

    // boundary first/last CSRs.
    for slot in 1..=max_slot {
        assert_eq!(
            mapped.boundary_first(slot).unwrap().as_deref(),
            Some(boundary.first_slots[(slot - 1) as usize].as_slice()),
        );
        assert_eq!(
            mapped.boundary_last(slot).unwrap().as_deref(),
            Some(boundary.last_slots[(slot - 1) as usize].as_slice()),
        );
    }

    // sections data round-trips exactly.
    let mapped_sections = mapped.sections_data().unwrap().expect("sections present");
    assert_eq!(*mapped_sections, sections);
    assert!(
        !mapped_sections.node_from_seq.is_empty(),
        "3-level corpus must populate node_from_seq"
    );
    assert!(!mapped_sections.seq_from_node.is_empty());
    assert!(!mapped_sections.sec1.is_empty());
    assert!(!mapped_sections.sec2.is_empty());

    // computed_feature surfaces levUp/levDown via the dispatcher too.
    match mapped.computed_feature("levUp").unwrap() {
        Some(context_fabric_core::ComputedFeatureData::LevUp(rows)) => assert_eq!(rows, lev_up),
        other => panic!("expected LevUp, got {other:?}"),
    }
    match mapped.computed_feature("levDown").unwrap() {
        Some(context_fabric_core::ComputedFeatureData::LevDown(rows)) => assert_eq!(rows, lev_down),
        other => panic!("expected LevDown, got {other:?}"),
    }
}

#[test]
fn pre_v3_cache_still_loads_with_fallback() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("three_level");
    write_three_level_corpus(&source);
    let cache = temp.path().join("three_level.cfr");
    compile_features(&source, &cache, &[]).unwrap();

    // Determine where the v3 region begins, then truncate it away to simulate
    // an old pre-v3 cache.
    let v3_start = {
        let mapped = MappedCompiledCorpus::open(&cache).unwrap();
        mapped.metadata().v3_start.expect("v3 region present")
    };
    let bytes = fs::read(&cache).unwrap();
    let legacy = temp.path().join("legacy.cfr");
    fs::write(&legacy, &bytes[..v3_start]).unwrap();

    let mapped = MappedCompiledCorpus::open(&legacy).unwrap();
    let meta = mapped.metadata();
    assert!(meta.v3_start.is_none(), "truncated cache has no v3 region");
    assert!(meta.lev_up_start.is_none());
    assert!(meta.sections_start.is_none());

    // Accessors return None (fallback) rather than erroring.
    assert_eq!(mapped.lev_up_row(7).unwrap(), None);
    assert_eq!(mapped.lev_down_row(9).unwrap(), None);
    assert_eq!(mapped.boundary_first(1).unwrap(), None);
    assert_eq!(mapped.boundary_last(1).unwrap(), None);
    assert_eq!(mapped.sections_data().unwrap(), None);

    // Core feature reads still work on the legacy cache.
    let otype = mapped.string_pool_node_feature("otype").unwrap().unwrap();
    assert_eq!(otype.str_value(7).unwrap(), Some("verse"));
    let oslots = mapped.edge_feature("oslots").unwrap().unwrap();
    assert_eq!(oslots.f(7).unwrap(), vec![1, 2, 3]);
    // boundary computed feature falls back to the scan path and still completes.
    assert!(mapped.computed_feature("boundary").unwrap().is_some());
}

#[test]
fn oslots_s_returns_slot_identity_for_slots() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("mini");
    copy_mini_corpus(&source);
    let cache = temp.path().join("mini.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache).unwrap();

    let oslots = mapped.edge_feature("oslots").unwrap().unwrap();
    // A slot is its own slot.
    assert_eq!(oslots.s(1).unwrap(), vec![1]);
    assert_eq!(oslots.s(3).unwrap(), vec![3]);
    // Non-slot nodes return their real slots.
    assert_eq!(oslots.s(6).unwrap(), vec![1, 2, 3]);
    // forward/f do NOT apply slot identity (raw adjacency).
    assert!(oslots.forward(1).unwrap().is_empty());
    assert!(oslots.f(1).unwrap().is_empty());
}

#[test]
fn node_feature_s_uses_canonical_rank_order() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("mini");
    copy_mini_corpus(&source);
    // A feature shared by a sentence (node 8, slots 1-5) and a phrase
    // (node 6, slots 1-3). The superset (8) ranks before the subset (6), so
    // canonical order [8, 6] differs from node-id order [6, 8].
    fs::write(
        source.join("tag.tf"),
        "@node\n@valueType=str\n\n6\tshared\n8\tshared\n",
    )
    .unwrap();
    let cache = temp.path().join("mini.cfr");
    compile_features(&source, &cache, &[]).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache).unwrap();
    let corpus = Corpus::load(&source).unwrap();

    let tag = mapped.string_pool_node_feature("tag").unwrap().unwrap();
    let mapped_nodes = tag.s("shared").unwrap();
    let canonical = corpus.node_feature("tag").unwrap().s(&FeatureValue::string("shared"));

    // Mapped matches the in-memory canonical (rank) order.
    assert_eq!(mapped_nodes, canonical);
    // And the order is genuinely rank-based, not ascending node id.
    let mut ascending = mapped_nodes.clone();
    ascending.sort_unstable();
    assert_ne!(
        mapped_nodes, ascending,
        "fixture must exercise non-ascending canonical order"
    );
    assert_eq!(mapped_nodes, vec![8, 6]);
}

#[test]
fn edge_value_index_distinguishes_zero_from_missing() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("mini");
    copy_mini_corpus(&source);
    let cache = temp.path().join("mini.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache).unwrap();

    let distance = mapped.edge_feature("distance").unwrap().unwrap();
    assert!(distance.has_edge_values());

    // Explicit zero is preserved (not conflated with missing).
    assert_eq!(distance.edge_value(1, 2).unwrap(), Some(MappedNodeValue::Int(0)));
    assert_eq!(distance.edge_value(3, 4).unwrap(), Some(MappedNodeValue::Int(0)));
    assert_eq!(distance.edge_value(6, 7).unwrap(), Some(MappedNodeValue::Int(0)));
    // Non-zero values.
    assert_eq!(distance.edge_value(1, 3).unwrap(), Some(MappedNodeValue::Int(5)));
    assert_eq!(distance.edge_value(4, 5).unwrap(), Some(MappedNodeValue::Int(10)));
    // Edge present but value absent -> None.
    assert!(distance.forward(2).unwrap().contains(&3));
    assert_eq!(distance.edge_value(2, 3).unwrap(), None);
    // No such edge -> None.
    assert_eq!(distance.edge_value(1, 4).unwrap(), None);

    // Binary search yields the same answers when queried repeatedly (index is
    // cached after first build).
    assert_eq!(distance.edge_value(1, 2).unwrap(), Some(MappedNodeValue::Int(0)));
    assert_eq!(distance.edge_value(2, 3).unwrap(), None);
}

#[test]
fn repeated_node_feature_calls_hit_cache_and_stay_consistent() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("mini");
    copy_mini_corpus(&source);
    let cache = temp.path().join("mini.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache).unwrap();

    // First call populates the cache; subsequent calls reuse the Arc'd parts.
    let first = mapped.string_pool_node_feature("word").unwrap().unwrap();
    let first_hello = first.str_value(1).unwrap().map(str::to_string);
    drop(first);

    for _ in 0..5 {
        let view = mapped.string_pool_node_feature("word").unwrap().unwrap();
        assert_eq!(view.str_value(1).unwrap().map(str::to_string), first_hello);
        assert_eq!(view.s("hello").unwrap(), vec![1]);
    }

    // Edge views likewise.
    let edge_first = mapped.edge_feature("parent").unwrap().unwrap();
    let targets = edge_first.forward(1).unwrap();
    drop(edge_first);
    for _ in 0..5 {
        let view = mapped.edge_feature("parent").unwrap().unwrap();
        assert_eq!(view.forward(1).unwrap(), targets);
    }
}

/// Opt-in full-BHSA compile + size + v3 + timing probe. Run with
/// `CF_BHSA_PERF=1 cargo test --release --test mapped_perf -- --nocapture
/// bhsa --ignored` (requires the BHSA corpus checked out).
#[test]
#[ignore]
fn bhsa_compile_size_and_timing_probe() {
    if std::env::var("CF_BHSA_PERF").is_err() {
        eprintln!("skipping; set CF_BHSA_PERF=1 to run");
        return;
    }
    let tf = std::env::var("CF_BHSA_TF")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benchmarks/.corpora/bhsa/tf")
        });
    if !tf.exists() {
        eprintln!("skipping; BHSA corpus not found at {}", tf.display());
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let cache = temp.path().join("bhsa-v3.cfr");

    let start = Instant::now();
    compile_features(&tf, &cache, &[]).unwrap();
    let compile_ms = start.elapsed().as_secs_f64() * 1000.0;
    let size_mb = fs::metadata(&cache).unwrap().len() as f64 / 1.0e6;
    println!("BHSA compile: {compile_ms:.0} ms, cache size {size_mb:.1} MB");

    let mapped = MappedCompiledCorpus::open(&cache).unwrap();
    let meta = mapped.metadata();
    println!(
        "v3 present={} lev_up={:?} lev_down={:?} boundary_first={:?} sections={:?}",
        meta.v3_start.is_some(),
        meta.lev_up_start,
        meta.lev_down_start,
        meta.boundary_first_start,
        meta.sections_start,
    );
    assert!(meta.v3_start.is_some());

    // Repeated string-pool feature value lookups: warm then measure.
    let word = mapped.string_pool_node_feature("g_word").unwrap();
    if let Some(view) = word {
        let _ = view.str_value(1).unwrap();
        let start = Instant::now();
        let n = 200_000;
        let mut sink = 0usize;
        for node in 0..n {
            if view.str_value((node % 400_000) + 1).unwrap().is_some() {
                sink += 1;
            }
        }
        let per = start.elapsed().as_nanos() as f64 / n as f64;
        println!("g_word.v: {per:.0} ns/call ({sink} hits)");
    }

    // Edge targets without rank materialization.
    if let Some(mother) = mapped.edge_feature("mother").unwrap() {
        let start = Instant::now();
        let mut total = 0usize;
        for src in 1..=100_000u32 {
            total += mother.f(src).unwrap().len();
        }
        let per = start.elapsed().as_nanos() as f64 / 100_000.0;
        println!("mother.f: {per:.0} ns/call ({total} edges)");
    }

    // lev_up / boundary accessors.
    let start = Instant::now();
    let mut sum = 0usize;
    for node in 1..=200_000u32 {
        sum += mapped.lev_up_row(node).unwrap().map(|r| r.len()).unwrap_or(0);
    }
    let per = start.elapsed().as_nanos() as f64 / 200_000.0;
    println!("lev_up_row: {per:.0} ns/call ({sum} embedders)");
}

#[test]
fn timing_probe_string_pool_lookup_and_edge_targets() {
    // Not a hard assertion gate (timing varies by machine); prints evidence
    // that cached lookups are microsecond-scale and edge targets need no full
    // rank materialization.
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("mini");
    copy_mini_corpus(&source);
    let cache = temp.path().join("mini.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache).unwrap();

    // Warm the cache.
    let _ = mapped.string_pool_node_feature("word").unwrap().unwrap();

    let view = mapped.string_pool_node_feature("word").unwrap().unwrap();
    let start = Instant::now();
    let iterations = 100_000;
    let mut sink = 0usize;
    for _ in 0..iterations {
        if view.str_value(1).unwrap().is_some() {
            sink += 1;
        }
    }
    let per_call = start.elapsed().as_nanos() as f64 / iterations as f64;
    assert_eq!(sink, iterations);
    println!("string-pool v lookup: {per_call:.1} ns/call (cached)");

    let oslots = mapped.edge_feature("oslots").unwrap().unwrap();
    let start = Instant::now();
    let mut total = 0usize;
    for _ in 0..iterations {
        total += oslots.f(8).unwrap().len();
    }
    let per_call = start.elapsed().as_nanos() as f64 / iterations as f64;
    assert!(total > 0);
    println!("edge targets lookup: {per_call:.1} ns/call (no rank materialization)");
}
