//! Phase 3 (Agent 3A) tests for `mapped_sections.rs` locality + sections.
//!
//! These pin the mapped path (levUp/levDown/boundary/sections CSRs) against
//! the in-memory `Corpus` locality reference on tiny fixture corpora compiled
//! to a temp `.cfr`.
//!
//! For `u`/`d`/`n`/`p`/`i` both must agree exactly. For `i` the result is
//! additionally asserted to be in ascending canonical (rank) order, matching
//! text-fabric `L.i` (`tf/core/locality.py` `i` returns `sortNodes(result -
//! {n})`, i.e. ascending canonical order with no reversal). The old `.reverse()`
//! was removed from both the mapped and in-memory intersecting paths.

use std::fs;
use std::path::{Path, PathBuf};

use context_fabric_core::{
    Corpus, FeatureValue, MappedCompiledCorpus, MappedSections, SectionOptions, compile_features,
};

fn mini_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini_corpus")
}

/// 3-level (book > chapter > verse > word) corpus that fully exercises the
/// `CFRSECT1` sections data (sec1, sec2). Mirrors the fixture in `mapped_perf`.
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

/// Compiles `source` to a `.cfr`, then returns the open mapped corpus and the
/// in-memory reference. The tempdir is returned so it outlives the mmap.
struct Harness {
    _temp: tempfile::TempDir,
    v3: MappedCompiledCorpus,
    corpus: Corpus,
}

fn build_harness(write_corpus: impl FnOnce(&Path)) -> Harness {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    write_corpus(&source);

    let cache = temp.path().join("full.cfr");
    compile_features(&source, &cache, &[]).unwrap();

    Harness {
        v3: MappedCompiledCorpus::open(&cache).unwrap(),
        corpus: Corpus::load(&source).unwrap(),
        _temp: temp,
    }
}

/// Asserts a node list is in strictly ascending canonical (rank) order.
fn assert_ascending_rank(mapped: &MappedCompiledCorpus, nodes: &[u32], context: &str) {
    let ranks: Vec<u32> = nodes
        .iter()
        .map(|node| mapped.sort_key(*node).unwrap().expect("node has a rank"))
        .collect();
    for window in ranks.windows(2) {
        assert!(
            window[0] < window[1],
            "expected ascending canonical order for {context}, got ranks {ranks:?} (nodes {nodes:?})",
        );
    }
}

/// Drives u/d/n/p/i across every node and every otype filter, asserting the
/// mapped and in-memory paths agree (with the documented L.i caveat).
fn assert_locality_parity(harness: &Harness, node_types: &[Option<&str>]) {
    let sv3 = MappedSections::new(&harness.v3).unwrap();
    let corpus = &harness.corpus;
    let max_node = corpus.max_node();

    for node in 1..=max_node {
        for ot in node_types {
            let ot = *ot;
            let label = format!("node {node}, otype {ot:?}");

            // Upward (embedders).
            let u_v3 = sv3.u(node, ot).unwrap();
            assert_eq!(u_v3, corpus.u(node, ot), "L.u v3 != in-memory ({label})");

            // Downward (embeddees).
            let d_v3 = sv3.d(node, ot).unwrap();
            assert_eq!(d_v3, corpus.d(node, ot), "L.d v3 != in-memory ({label})");

            // Next.
            let n_v3 = sv3.n(node, ot).unwrap();
            assert_eq!(n_v3, corpus.n(node, ot), "L.n v3 != in-memory ({label})");

            // Previous.
            let p_v3 = sv3.p(node, ot).unwrap();
            assert_eq!(p_v3, corpus.p(node, ot), "L.p v3 != in-memory ({label})");

            // Intersecting: both paths agree, in ascending canonical order
            // (TF `L.i`; the old `.reverse()` was removed from both paths).
            let i_v3 = sv3.i(node, ot).unwrap();
            assert_eq!(i_v3, corpus.i(node, ot), "L.i v3 != in-memory ({label})");
            assert_ascending_rank(&harness.v3, &i_v3, &label);
        }
    }
}

#[test]
fn mini_corpus_locality_parity_across_paths() {
    let harness = build_harness(|dir| {
        fs::create_dir_all(dir).unwrap();
        for entry in fs::read_dir(mini_corpus_dir()).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("tf") {
                fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
            }
        }
    });
    assert_locality_parity(
        &harness,
        &[None, Some("word"), Some("phrase"), Some("sentence")],
    );
}

#[test]
fn three_level_corpus_locality_parity_across_paths() {
    let harness = build_harness(write_three_level_corpus);
    assert_locality_parity(
        &harness,
        &[
            None,
            Some("word"),
            Some("verse"),
            Some("chapter"),
            Some("book"),
        ],
    );
}

#[test]
fn l_i_is_ascending_canonical_order() {
    // Direct assertion of the L.i order fix: text-fabric returns the intersectors
    // in canonical ascending order (no reversal). For the 3-level corpus, the
    // sentence/verse-level nodes intersect words and one another.
    let harness = build_harness(write_three_level_corpus);
    let sv3 = MappedSections::new(&harness.v3).unwrap();

    // Book node 10 (slots 1-6) intersects chapter 9, verses 7,8 and all 6 words.
    let i_book = sv3.i(10, None).unwrap();
    assert!(!i_book.is_empty());
    assert!(!i_book.contains(&10), "L.i never includes the node itself");
    assert_ascending_rank(&harness.v3, &i_book, "L.i(book)");

    // Verse 7 (slots 1-3) intersects book 10, chapter 9 and words 1,2,3.
    let i_verse = sv3.i(7, None).unwrap();
    assert!(i_verse.contains(&1) && i_verse.contains(&2) && i_verse.contains(&3));
    assert!(i_verse.contains(&9) && i_verse.contains(&10));
    assert!(!i_verse.contains(&7));
    assert_ascending_rank(&harness.v3, &i_verse, "L.i(verse)");
}

#[test]
fn node_from_section_round_trips_three_level() {
    let harness = build_harness(write_three_level_corpus);
    let sv3 = MappedSections::new(&harness.v3).unwrap();
    let corpus = &harness.corpus;

    // Every section node (book, chapter, verse) must round-trip through its own
    // section tuple, on the mapped and in-memory paths. (Slots also yield
    // a section tuple, but `node_from_section` resolves to the deepest *section*
    // node, not the slot, so only section-type nodes round-trip to themselves.)
    let section_types = corpus.section_types();
    for node in 1..=corpus.max_node() {
        if !corpus
            .node_type(node)
            .is_some_and(|ty| section_types.iter().any(|st| st == ty))
        {
            continue;
        }
        let tuple = corpus.section_from_node(node, &SectionOptions::default());
        if tuple.is_empty() || tuple.iter().any(Option::is_none) {
            continue;
        }
        let section: Vec<FeatureValue> = tuple.into_iter().map(Option::unwrap).collect();

        assert_eq!(
            sv3.node_from_section(&section).unwrap(),
            Some(node),
            "v3 node_from_section round trip failed for node {node} ({section:?})",
        );
        assert_eq!(
            corpus.node_from_section(&section),
            Some(node),
            "in-memory node_from_section round trip failed for node {node} ({section:?})",
        );
    }

    // Explicit lookups exercising each level.
    let genesis = FeatureValue::string("Genesis");
    assert_eq!(
        sv3.node_from_section(std::slice::from_ref(&genesis))
            .unwrap(),
        Some(10),
    );
    assert_eq!(
        sv3.node_from_section(&[genesis.clone(), FeatureValue::Int(1)])
            .unwrap(),
        Some(9),
    );
    assert_eq!(
        sv3.node_from_section(&[genesis.clone(), FeatureValue::Int(1), FeatureValue::Int(2)])
            .unwrap(),
        Some(8),
    );

    // Unknown sections return None.
    let missing = FeatureValue::string("Nonexistent");
    assert_eq!(
        sv3.node_from_section(std::slice::from_ref(&missing))
            .unwrap(),
        None,
    );
    assert_eq!(
        sv3.node_from_section(&[genesis, FeatureValue::Int(99)])
            .unwrap(),
        None,
    );
}

#[test]
fn node_from_section_round_trips_mini_two_level() {
    let harness = build_harness(|dir| {
        fs::create_dir_all(dir).unwrap();
        for entry in fs::read_dir(mini_corpus_dir()).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("tf") {
                fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
            }
        }
    });
    let sv3 = MappedSections::new(&harness.v3).unwrap();
    let corpus = &harness.corpus;

    let section_types = corpus.section_types();
    for node in 1..=corpus.max_node() {
        if !corpus
            .node_type(node)
            .is_some_and(|ty| section_types.iter().any(|st| st == ty))
        {
            continue;
        }
        let tuple = corpus.section_from_node(node, &SectionOptions::default());
        if tuple.is_empty() || tuple.iter().any(Option::is_none) {
            continue;
        }
        let section: Vec<FeatureValue> = tuple.into_iter().map(Option::unwrap).collect();
        assert_eq!(sv3.node_from_section(&section).unwrap(), Some(node));
        assert_eq!(corpus.node_from_section(&section), Some(node));
    }
}
