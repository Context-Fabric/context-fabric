//! Loading `.tf` sources into the in-memory corpus that compilation reads.

use std::fs;

mod common;

use common::*;
use context_fabric_core::*;

#[test]
fn corpus_loading_reports_clean_errors_for_missing_or_empty_locations() {
    let missing_path = tempfile::tempdir().unwrap().path().join("missing-corpus");
    let missing_error = Corpus::load(&missing_path).unwrap_err();
    assert!(
        matches!(missing_error, CfError::Io { .. }),
        "missing corpus path should produce an I/O error, got {missing_error:?}"
    );

    let empty_dir = tempfile::tempdir().unwrap();
    let empty_error = Fabric::new(empty_dir.path()).load_all().unwrap_err();
    assert!(matches!(
        empty_error,
        CfError::MissingFeature(feature) if feature == "otype"
    ));
}

#[test]
fn selective_load_always_includes_warp_features_and_configs() {
    let corpus = Corpus::load_features(mini_corpus_dir(), &["word"]).unwrap();

    assert!(corpus.node_feature("word").is_some());
    assert!(corpus.node_feature("otype").is_some());
    assert!(corpus.edge_feature("oslots").is_some());
    assert!(corpus.config_feature("otext").is_some());
    assert!(corpus.node_feature("pos").is_none());
    assert!(corpus.edge_feature("parent").is_none());
}

#[test]
fn oslots_items_returns_empty_when_there_are_no_non_slot_nodes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nword\n",
    )
    .unwrap();
    fs::write(dir.path().join("oslots.tf"), "@edge\n@valueType=int\n\n").unwrap();

    let corpus = Corpus::load(dir.path()).unwrap();

    assert_eq!(corpus.oslots_items(), Vec::<(u32, Vec<u32>)>::new());
}

#[test]
fn explores_tf_directory_without_materializing_features() {
    let inventory = explore_feature_paths([mini_corpus_dir()]).unwrap();

    assert!(inventory.nodes.contains(&"otype".to_string()));
    assert!(inventory.nodes.contains(&"word".to_string()));
    assert!(inventory.nodes.contains(&"pos".to_string()));
    assert!(inventory.edges.contains(&"oslots".to_string()));
    assert!(inventory.edges.contains(&"parent".to_string()));
    assert!(inventory.configs.contains(&"otext".to_string()));
    assert!(inventory.computeds.contains(&"rank".to_string()));
    assert!(!inventory.nodes.contains(&"parent".to_string()));
    assert!(!inventory.edges.contains(&"word".to_string()));

    assert!(explore_feature_paths([repo_path("libs/core/tests/fixtures/invalid")]).is_err());
}
