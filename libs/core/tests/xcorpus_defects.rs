//! Regression tests for the five cross-corpus defects fixed against
//! text-fabric 13.0.19 parity:
//!   1. multi-location `Fabric::compile` (warp features in an earlier location),
//!   2. text-format `/` fallback treating a present empty string as absent,
//!   3. `@levelConstraints` ignored when computing otype ranks,
//!   4. section-0 language feature restricted to the `sectionFeatures[0]` family,
//!   5. `node_from_section` language override + immediate-miss fast path.
//!
//! Each uses a tiny `.tf` fixture compiled to a temp `.cfr` (no network, no large
//! corpora); the corpus-dependent halves of #4/#5 are covered by the env-gated
//! python tests on quran/bhsa.

use std::fs;
use std::path::Path;

use context_fabric_core::{
    Corpus, Fabric, FeatureValue, MappedCompiledCorpus, MappedSections, MappedText, compile_features,
    precompute,
};

fn write_files(dir: &Path, files: &[(&str, &str)]) {
    fs::create_dir_all(dir).unwrap();
    for (name, contents) in files {
        fs::write(dir.join(name), contents).unwrap();
    }
}

// ---------------------------------------------------------------------------
// BUG-1: multi-location compile must merge warp features from every location.
// ---------------------------------------------------------------------------

#[test]
fn multi_location_compile_merges_warp_and_added_features() {
    let temp = tempfile::tempdir().unwrap();
    // Base location carries the warp features (otype/oslots) + text.
    let base = temp.path().join("base");
    write_files(
        &base,
        &[
            ("otype.tf", "@node\n@valueType=str\n\nword\nword\n3\tphrase\n"),
            ("oslots.tf", "@edge\n@valueType=int\n\n3\t1-2\n"),
            ("word.tf", "@node\n@valueType=str\n\na\nb\n"),
            ("otext.tf", "@config\n@fmt:text-orig-full={word}\n"),
        ],
    );
    // Module location adds an edge feature only (no otype of its own).
    let module = temp.path().join("module");
    write_files(
        &module,
        &[("crossref.tf", "@edge\n@edgeValues\n@valueType=int\n\n3\t3\t7\n")],
    );

    // Single-location compile of the module alone must fail: it has no otype.
    let lone = temp.path().join("lone.cfr");
    assert!(
        compile_features(&module, &lone, &[]).is_err(),
        "module-only compile should fail without otype",
    );

    // Multi-location compile must succeed and merge both locations.
    let cache = temp.path().join("merged.cfr");
    let fabric = Fabric::from_locations([base.clone(), module.clone()]);
    fabric.compile(&cache, Vec::<String>::new()).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache).unwrap();
    let node_features = mapped.all_node_features(true);
    assert!(node_features.iter().any(|name| name == "otype"));
    let edge_features = mapped.all_edge_features(true);
    assert!(
        edge_features.iter().any(|name| name == "crossref"),
        "crossref from the second location must be merged: {edge_features:?}",
    );
    assert!(edge_features.iter().any(|name| name == "oslots"));
}

// ---------------------------------------------------------------------------
// BUG-2: a present empty-string feature value must be emitted, not skipped.
// ---------------------------------------------------------------------------

fn build_qere_corpus(dir: &Path) {
    write_files(
        dir,
        &[
            // words 1,2 (slots) and phrase 3.
            ("otype.tf", "@node\n@valueType=str\n\nword\nword\n3\tphrase\n"),
            ("oslots.tf", "@edge\n@valueType=int\n\n3\t1-2\n"),
            ("word.tf", "@node\n@valueType=str\n\nA\nB\n"),
            // node 1 has an explicit EMPTY qtrailer; node 2 has none.
            ("qtrailer.tf", "@node\n@valueType=str\n\n1\t\n"),
            // trailer present for both: node1 = "-", node2 = ".".
            ("trailer.tf", "@node\n@valueType=str\n\n1\t-\n2\t.\n"),
            (
                "otext.tf",
                "@config\n@fmt:text-orig-full={word}{qtrailer/trailer}\n",
            ),
        ],
    );
}

#[test]
fn text_format_keeps_present_empty_value_instead_of_falling_back() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("qere");
    build_qere_corpus(&source);
    let cache = temp.path().join("qere.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache).unwrap();
    let text = MappedText::new(&mapped).unwrap();

    // Node 1: qtrailer is present-but-empty -> emit "" (NOT the trailer "-").
    assert_eq!(text.text(1, None).unwrap(), "A");
    // Node 2: qtrailer absent -> fall back to trailer ".".
    assert_eq!(text.text(2, None).unwrap(), "B.");
    // Phrase renders both slots concatenated.
    assert_eq!(text.text(3, None).unwrap(), "AB.");

    // The in-memory Corpus text path shares the same fallback semantics.
    let corpus = Corpus::load(&source).unwrap();
    assert_eq!(corpus.text(1, None), "A");
    assert_eq!(corpus.text(2, None), "B.");
}

#[test]
fn text_context_preserves_present_empty_fallback() {
    // The cached `TextContext` (compiled op-lists + owned feature handles) is the
    // path the Python `T.text` binding now uses; it must reproduce the BUG-2
    // present-empty-vs-absent fallback byte-for-byte.
    use std::sync::Arc;

    use context_fabric_core::corpus::TextOptions;
    use context_fabric_core::mapped_text::TextContext;

    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("qere");
    build_qere_corpus(&source);
    let cache = temp.path().join("qere.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    let mapped = Arc::new(MappedCompiledCorpus::open(&cache).unwrap());
    let context = TextContext::new(&mapped).unwrap();
    let none = TextOptions::new(None::<String>, None);

    // Same three assertions as the MappedText path, via the cached context.
    assert_eq!(context.text_with_options(1, &none).unwrap(), "A");
    assert_eq!(context.text_with_options(2, &none).unwrap(), "B.");
    assert_eq!(context.text_with_options(3, &none).unwrap(), "AB.");
}

// ---------------------------------------------------------------------------
// BUG-3: @levelConstraints must override size-based otype ranking.
// ---------------------------------------------------------------------------

fn level_corpus_files(constraint: Option<&str>) -> Vec<(&'static str, String)> {
    let otext = match constraint {
        Some(spec) => format!("@config\n@fmt:text-orig-full={{word}}\n@levelConstraints={spec}\n"),
        None => "@config\n@fmt:text-orig-full={word}\n".to_string(),
    };
    vec![
        (
            "otype.tf",
            // slots 1-4, then big (slots 1-4, avg 4) and two small (avg 2).
            "@node\n@valueType=str\n\nword\nword\nword\nword\n5\tbig\n6\tsmall\n7\tsmall\n"
                .to_string(),
        ),
        (
            "oslots.tf",
            "@edge\n@valueType=int\n\n5\t1-4\n6\t1-2\n7\t3-4\n".to_string(),
        ),
        ("word.tf", "@node\n@valueType=str\n\na\nb\nc\nd\n".to_string()),
        ("otext.tf", otext),
    ]
}

fn compile_level_corpus(constraint: Option<&str>) -> (tempfile::TempDir, MappedCompiledCorpus) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("levels");
    fs::create_dir_all(&source).unwrap();
    for (name, contents) in level_corpus_files(constraint) {
        fs::write(source.join(name), contents).unwrap();
    }
    let cache = temp.path().join("levels.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache).unwrap();
    (temp, mapped)
}

#[test]
fn level_constraints_override_size_based_otype_rank() {
    // Without a constraint, the bigger average type ("big") outranks "small".
    let (_t0, plain) = compile_level_corpus(None);
    let ranks_plain = plain.otype_rank().unwrap();
    assert!(
        ranks_plain["big"] > ranks_plain["small"],
        "size-only ranking should put big above small: {ranks_plain:?}",
    );

    // With "big < small", the constraint flips it (big now ranks below small),
    // matching TF's prepare.py despite big's larger average size.
    let (_t1, constrained) = compile_level_corpus(Some("big < small"));
    let ranks = constrained.otype_rank().unwrap();
    assert!(
        ranks["big"] < ranks["small"],
        "levelConstraints big<small should rank big below small: {ranks:?}",
    );

    // The in-memory Corpus path (used while compiling) honors it too.
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("levels");
    fs::create_dir_all(&source).unwrap();
    for (name, contents) in level_corpus_files(Some("big < small")) {
        fs::write(source.join(name), contents).unwrap();
    }
    let corpus = Corpus::load(&source).unwrap();
    let mem_ranks = corpus.otype_rank();
    assert!(mem_ranks["big"] < mem_ranks["small"], "{mem_ranks:?}");
}

#[test]
fn apply_level_constraints_matches_tf_pop_insert_semantics() {
    // Biggest-first order with slot last, as TF's `result` is built.
    let mut rows = vec!["big", "small", "word"];
    precompute::apply_level_constraints(&mut rows, "big < small", |row| row);
    assert_eq!(rows, vec!["small", "big", "word"]);

    // Multiple `;`-separated constraints are applied left to right, each
    // recomputing positions (TF rebuilds `resultIndex` after every move).
    let mut multi = vec!["a", "b", "c", "slot"];
    precompute::apply_level_constraints(&mut multi, "a < c ; b < c", |row| row);
    // a<c: pop a@0, insert@2 -> [b, c, a, slot]; then b<c: pop b@0, insert@1 ->
    // [c, b, a, slot].
    assert_eq!(multi, vec!["c", "b", "a", "slot"]);

    // A constraint naming an unknown type is a no-op.
    let mut unknown = vec!["x", "y", "slot"];
    precompute::apply_level_constraints(&mut unknown, "x < zzz", |row| row);
    assert_eq!(unknown, vec!["x", "y", "slot"]);
}

// ---------------------------------------------------------------------------
// BUG-4 / BUG-5 (core): node_from_section language override + immediate miss.
// ---------------------------------------------------------------------------

fn build_named_section_corpus(dir: &Path) {
    write_files(
        dir,
        &[
            // words 1-2 (slots), aya 3, sura 4.
            (
                "otype.tf",
                "@node\n@valueType=str\n\nword\nword\n3\taya\n4\tsura\n",
            ),
            ("oslots.tf", "@edge\n@valueType=int\n\n3\t1-2\n4\t1-2\n"),
            ("word.tf", "@node\n@valueType=str\n\na\nb\n"),
            // base section feature `number` (int) on sura(4) and aya(3).
            ("number.tf", "@node\n@valueType=int\n\n3\t1\n4\t1\n"),
            // English-name variant carrying a languageCode (like quran name@en).
            (
                "ename.tf",
                "@node\n@valueType=str\n@languageCode=en\n\n4\tThe Opening\n",
            ),
            (
                "otext.tf",
                "@config\n@fmt:text-orig-full={word}\n\
                 @sectionTypes=sura,aya\n@sectionFeatures=number,number\n",
            ),
        ],
    );
}

#[test]
fn node_from_section_language_override_and_fast_miss() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("named");
    build_named_section_corpus(&source);
    let cache = temp.path().join("named.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache).unwrap();
    let sections = MappedSections::new(&mapped).unwrap();

    // Default (configured `number`) resolution: sura 1 -> node 4.
    assert_eq!(
        sections.node_from_section(&[FeatureValue::Int(1)]).unwrap(),
        Some(4),
    );
    // Language override: resolve the section-0 node via the `ename` feature.
    assert_eq!(
        sections
            .node_from_section_langed(&[FeatureValue::string("The Opening")], Some("ename"))
            .unwrap(),
        Some(4),
    );
    // A miss returns None immediately (v3 index present) — no linear rescan.
    assert_eq!(
        sections.node_from_section(&[FeatureValue::Int(999)]).unwrap(),
        None,
    );
    assert_eq!(
        sections
            .node_from_section(&[FeatureValue::Int(1), FeatureValue::Int(999)])
            .unwrap(),
        None,
    );
}
