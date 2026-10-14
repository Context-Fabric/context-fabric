//! Smoke tests on the real BHSA corpus (skipped when it is absent; see
//! `common::optional_corpus_tf`).

mod common;

use common::*;
use context_fabric_core::*;

#[test]
fn loads_bhsa_and_runs_lexical_queries() {
    let Some(path) = bhsa_tf() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("bhsa-lexical.cfr");
    compile_features(path, &cache_path, &["otype", "oslots", "sp"]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();

    assert_eq!(mapped.nodes_of_type("word").unwrap().len(), 426_590);
    assert_eq!(mapped.nodes_of_type("book").unwrap().len(), 39);
    assert_eq!(mapped.max_slot().unwrap(), 426_590);
    let otype = mapped.string_pool_node_feature("otype").unwrap().unwrap();
    assert_eq!(otype.str_value(426_591).unwrap(), Some("book"));

    let verbs = MappedSearch::new(&mapped)
        .unwrap()
        .search("word sp=verb", Some(10))
        .unwrap();
    assert_eq!(verbs.len(), 10);
    let sp = mapped.string_pool_node_feature("sp").unwrap().unwrap();
    for row in verbs {
        assert_eq!(sp.str_value(row[0]).unwrap(), Some("verb"));
    }
}

#[test]
fn renders_bhsa_text_from_otext_format_features() {
    let Some(path) = bhsa_tf() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("bhsa-text.cfr");
    compile_features(
        path,
        &cache_path,
        &[
            "qere_utf8",
            "g_word_utf8",
            "qere_trailer_utf8",
            "trailer_utf8",
            "qere",
            "g_word",
            "qere_trailer",
            "trailer",
        ],
    )
    .unwrap();
    let mapped = std::sync::Arc::new(MappedCompiledCorpus::open(&cache_path).unwrap());
    let renderer = TextRenderer::new(&mapped).unwrap();
    let text = |format: Option<&str>| renderer.text(1, &TextOptions::new(format, None)).unwrap();

    let original = text(Some("text-orig-full"));
    assert!(!original.is_empty());
    assert_eq!(text(None), original);
    assert_eq!(text(Some("fmt:text-orig-full")), original);
    let transliterated = text(Some("text-trans-full"));
    assert!(!transliterated.is_empty());
    assert_ne!(original, transliterated);
}

#[test]
fn runs_representative_bhsa_curated_query_shapes() {
    let Some(path) = bhsa_tf() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let cache_path = dir.path().join("bhsa-curated.cfr");
    compile_features(
        path,
        &cache_path,
        &[
            "otype", "oslots", "sp", "vt", "vs", "gn", "nu", "language", "function", "typ", "kind",
        ],
    )
    .unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let search = MappedSearch::new(&mapped).unwrap();

    let lexical = search.search("word sp=verb vt=perf", Some(5)).unwrap();
    assert_eq!(lexical.len(), 5);

    let structural = search
        .search(
            "
phrase typ=VP
  word sp=verb
",
            Some(5),
        )
        .unwrap();
    assert_eq!(structural.len(), 5);

    let quantified = search
        .search(
            "
phrase typ=NP
/with/
  word sp=art
/-/
",
            Some(5),
        )
        .unwrap();
    assert_eq!(quantified.len(), 5);
}
