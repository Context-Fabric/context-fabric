//! Parsing Text-Fabric `.tf` feature files.

use std::fs;

mod common;

use common::*;
use context_fabric_core::*;

/// Nodes carrying `value`, ascending.
fn nodes_with(feature: &NodeFeature, value: &str) -> Vec<u32> {
    let mut nodes = feature.by_value[&FeatureValue::string(value)].clone();
    nodes.sort_unstable();
    nodes
}

/// Targets of `source`, ascending.
fn targets(feature: &EdgeFeature, source: u32) -> Vec<u32> {
    let mut targets = feature.targets(source).unwrap_or_default().to_vec();
    targets.sort_unstable();
    targets
}

/// Sources with an edge to `target`, ascending.
fn sources(feature: &EdgeFeature, target: u32) -> Vec<u32> {
    let mut sources: Vec<u32> = feature
        .values
        .iter()
        .filter(|(_, targets)| targets.contains(&target))
        .map(|(source, _)| *source)
        .collect();
    sources.sort_unstable();
    sources
}

fn metadata_value<'a>(feature: &'a NodeFeature, key: &str) -> Option<&'a str> {
    feature.metadata.get(key).and_then(Option::as_deref)
}

#[test]
fn parses_implicit_node_feature_values() {
    let path = repo_path("libs/core/tests/fixtures/mini_corpus/word.tf");
    let feature = parse_tf_file(path).expect("word.tf should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.str_value(1), Some("hello"));
    assert_eq!(feature.str_value(5), Some("morning"));
}

#[test]
fn parses_explicit_node_feature_rows_and_resumes_implicit_numbering() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("explicit.tf");
    fs::write(
        &path,
        "@node\n@valueType=str\n\n5\tvalue5\n10\tvalue10\nimplicit_after\n",
    )
    .unwrap();

    let feature = parse_tf_file(path).expect("explicit node rows should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.str_value(5), Some("value5"));
    assert_eq!(feature.str_value(10), Some("value10"));
    assert_eq!(feature.str_value(11), Some("implicit_after"));
}

#[test]
fn parses_node_and_edge_specs_with_ranges_and_commas() {
    let dir = tempfile::tempdir().unwrap();

    let node_path = dir.path().join("ranges.tf");
    fs::write(
        &node_path,
        "@node\n@valueType=str\n\n1-3,5\tshared\n8\teight\nimplicit_after\n",
    )
    .unwrap();

    let feature = parse_tf_file(&node_path).expect("node ranges should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(nodes_with(&feature, "shared"), &[1, 2, 3, 5]);
    assert_eq!(feature.str_value(4), None);
    assert_eq!(feature.str_value(8), Some("eight"));
    assert_eq!(feature.str_value(9), Some("implicit_after"));

    let edge_path = dir.path().join("edges.tf");
    fs::write(
        &edge_path,
        "@edge\n@edgeValues\n@valueType=str\n\n1-2,4\t7-8,10\tlinked\n",
    )
    .unwrap();

    let feature = parse_tf_file(&edge_path).expect("edge ranges should parse");
    let TfFeature::Edge(feature) = feature else {
        panic!("expected edge feature");
    };
    assert_eq!(targets(&feature, 1), vec![7, 8, 10]);
    assert_eq!(targets(&feature, 2), vec![7, 8, 10]);
    assert_eq!(targets(&feature, 4), vec![7, 8, 10]);
    assert_eq!(
        feature.edge_value(4, 10),
        Some(&FeatureValue::string("linked"))
    );
    assert_eq!(
        sources(&feature, 8),
        vec![1, 2, 4],
        "target range should create inverse edges from every expanded source"
    );
}

#[test]
fn parses_descending_node_and_edge_ranges_like_python_loader() {
    let dir = tempfile::tempdir().unwrap();

    let node_path = dir.path().join("descending_ranges.tf");
    fs::write(
        &node_path,
        "@node\n@valueType=str\n\n5-3\treversed\n8\teight\nimplicit_after\n",
    )
    .unwrap();

    let feature = parse_tf_file(&node_path).expect("descending node ranges should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(nodes_with(&feature, "reversed"), &[3, 4, 5]);
    assert_eq!(feature.str_value(2), None);
    assert_eq!(feature.str_value(8), Some("eight"));
    assert_eq!(feature.str_value(9), Some("implicit_after"));

    let edge_path = dir.path().join("descending_edges.tf");
    fs::write(
        &edge_path,
        "@edge\n@edgeValues\n@valueType=str\n\n4-2\t8-6\treversed\n",
    )
    .unwrap();

    let feature = parse_tf_file(&edge_path).expect("descending edge ranges should parse");
    let TfFeature::Edge(feature) = feature else {
        panic!("expected edge feature");
    };
    assert_eq!(targets(&feature, 2), vec![6, 7, 8]);
    assert_eq!(targets(&feature, 3), vec![6, 7, 8]);
    assert_eq!(targets(&feature, 4), vec![6, 7, 8]);
    assert_eq!(
        feature.edge_value(2, 6),
        Some(&FeatureValue::string("reversed"))
    );
    assert_eq!(
        sources(&feature, 7),
        vec![2, 3, 4],
        "descending target range should create inverse edges from every normalized source"
    );
}

#[test]
fn parses_text_fabric_string_value_escapes_like_python_loader() {
    let dir = tempfile::tempdir().unwrap();

    let node_path = dir.path().join("escaped.tf");
    fs::write(
        &node_path,
        "@node\n@valueType=str\n\nhas\\ttab\nhas\\nnewline\nhas\\\\backslash\n",
    )
    .unwrap();

    let feature = parse_tf_file(&node_path).expect("escaped node values should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.str_value(1), Some("has\ttab"));
    assert_eq!(feature.str_value(2), Some("has\nnewline"));
    assert_eq!(feature.str_value(3), Some("has\\backslash"));

    let edge_path = dir.path().join("escaped_edges.tf");
    fs::write(
        &edge_path,
        "@edge\n@edgeValues\n@valueType=str\n\n1\t2\ta\\tb\n2\t3\ta\\nb\n3\t4\ta\\\\b\n",
    )
    .unwrap();

    let feature = parse_tf_file(&edge_path).expect("escaped edge values should parse");
    let TfFeature::Edge(feature) = feature else {
        panic!("expected edge feature");
    };
    assert_eq!(
        feature.edge_value(1, 2),
        Some(&FeatureValue::string("a\tb"))
    );
    assert_eq!(
        feature.edge_value(2, 3),
        Some(&FeatureValue::string("a\nb"))
    );
    assert_eq!(
        feature.edge_value(3, 4),
        Some(&FeatureValue::string("a\\b"))
    );
}

#[test]
fn text_fabric_value_escape_helpers_match_python_round_trip_behaviors() {
    assert_eq!(value_from_tf(r"a\tb"), "a\tb");
    assert_eq!(value_from_tf(r"a\nb"), "a\nb");
    assert_eq!(value_from_tf(r"a\\b"), r"a\b");
    assert_eq!(value_from_tf("hello world"), "hello world");

    assert_eq!(tf_from_value(&FeatureValue::string("a\tb")), r"a\tb");
    assert_eq!(tf_from_value(&FeatureValue::string("a\nb")), r"a\nb");
    assert_eq!(tf_from_value(&FeatureValue::string(r"a\b")), r"a\\b");
    assert_eq!(tf_from_value(&FeatureValue::Int(42)), "42");
}

#[test]
fn parses_explicit_ranges_in_otype() {
    let Some(path) = bhsa_tf().map(|path| path.join("otype.tf")) else {
        return;
    };
    let feature = parse_tf_file(path).expect("BHSA otype.tf should parse");
    let TfFeature::Node(feature) = feature else {
        panic!("expected node feature");
    };
    assert_eq!(feature.str_value(1), Some("word"));
    assert_eq!(feature.str_value(426590), Some("word"));
    assert_eq!(feature.str_value(426591), Some("book"));
}

#[test]
fn parses_edge_cases_and_valued_edges() {
    let empty = parse_tf_file(repo_path("libs/core/tests/fixtures/edge_cases/empty.tf")).unwrap();
    let TfFeature::Node(empty) = empty else {
        panic!("expected empty node feature");
    };
    assert!(empty.items().is_empty());

    let single = parse_tf_file(repo_path(
        "libs/core/tests/fixtures/edge_cases/single_node.tf",
    ))
    .unwrap();
    let TfFeature::Node(single) = single else {
        panic!("expected single-node feature");
    };
    assert_eq!(single.str_value(1), Some("only_value"));

    let unicode =
        parse_tf_file(repo_path("libs/core/tests/fixtures/edge_cases/unicode.tf")).unwrap();
    let TfFeature::Node(unicode) = unicode else {
        panic!("expected unicode node feature");
    };
    assert_eq!(unicode.str_value(2), Some("世界"));
    assert_eq!(unicode.str_value(4), Some("שלום"));
    assert_eq!(unicode.str_value(5), Some("🌍"));

    let relation = parse_tf_file(repo_path(
        "libs/core/tests/fixtures/mini_corpus/relation.tf",
    ))
    .unwrap();
    let TfFeature::Edge(relation) = relation else {
        panic!("expected relation edge feature");
    };
    assert!(relation.has_edge_values());
    assert_eq!(relation.edge_values.len(), 5);
    assert_eq!(
        relation
            .metadata
            .get("description")
            .and_then(Option::as_deref),
        Some("relation type between nodes (tests string edge values)")
    );
    assert_eq!(relation.metadata.get("edgeValues"), Some(&None));
    assert_eq!(targets(&relation, 1), vec![6]);
    assert_eq!(
        relation.edge_value(1, 6),
        Some(&FeatureValue::string("subject"))
    );
    assert_eq!(
        relation.edge_value(2, 6),
        Some(&FeatureValue::string("predicate"))
    );
    assert_eq!(relation.items()[0], (1, vec![6]));
    assert_eq!(sources(&relation, 6), vec![1, 2, 3]);

    let distance = parse_tf_file(repo_path(
        "libs/core/tests/fixtures/mini_corpus/distance.tf",
    ))
    .unwrap();
    let TfFeature::Edge(distance) = distance else {
        panic!("expected distance edge feature");
    };
    assert_eq!(distance.edge_value(1, 2), Some(&FeatureValue::Int(0)));
    assert_eq!(distance.edge_value(1, 3), Some(&FeatureValue::Int(5)));
    assert_eq!(targets(&distance, 2), vec![3]);
    assert_eq!(distance.edge_value(2, 3), None);
}

#[test]
fn parses_and_loads_config_features() {
    let parsed = parse_tf_file(repo_path("libs/core/tests/fixtures/mini_corpus/otext.tf")).unwrap();
    let TfFeature::Config { name, metadata } = parsed else {
        panic!("expected config feature");
    };
    assert_eq!(name, "otext");
    assert_eq!(
        metadata.get("sectionTypes").and_then(Option::as_deref),
        Some("sentence,phrase")
    );
    assert_eq!(
        metadata.get("sectionFeatures").and_then(Option::as_deref),
        Some("sentence_id,phrase_id")
    );
    assert_eq!(
        metadata
            .get("fmt:text-orig-full")
            .and_then(Option::as_deref),
        Some("{word}")
    );

    let corpus = Corpus::load(repo_path("libs/core/tests/fixtures/mini_corpus")).unwrap();
    let otext = corpus.config_feature("otext").unwrap();
    assert_eq!(
        otext.get("structureTypes").and_then(Option::as_deref),
        Some("")
    );
}

#[test]
fn parses_tf_metadata_without_loading_data_rows_like_python_meta_only() {
    let word_metadata =
        parse_tf_file_metadata(repo_path("libs/core/tests/fixtures/mini_corpus/word.tf")).unwrap();
    assert_eq!(word_metadata.name, "word");
    assert_eq!(word_metadata.kind, TfFeatureKind::Node);
    assert_eq!(
        word_metadata
            .metadata
            .get("description")
            .and_then(Option::as_deref),
        Some("word text")
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad_rows.tf");
    fs::write(
        &path,
        "@node\n@valueType=int\n@description=metadata only\n\nnot-an-int\n",
    )
    .unwrap();

    let metadata = parse_tf_file_metadata(&path).unwrap();
    assert_eq!(metadata.name, "bad_rows");
    assert_eq!(metadata.kind, TfFeatureKind::Node);
    assert_eq!(
        metadata
            .metadata
            .get("valueType")
            .and_then(Option::as_deref),
        Some("int")
    );
    assert!(parse_tf_file(&path).is_err());
}

#[test]
fn normalizes_desc_and_eg_metadata_like_python_format_meta() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\n",
    )
    .unwrap();
    fs::write(dir.path().join("oslots.tf"), "@edge\n@valueType=int\n\n").unwrap();
    fs::write(
        dir.path().join("gloss.tf"),
        "@node\n@valueType=str\n@desc=A feature\n@eg=example\n\nalpha\nbeta\n",
    )
    .unwrap();

    let metadata = parse_tf_file_metadata(dir.path().join("gloss.tf")).unwrap();
    assert_eq!(
        metadata
            .metadata
            .get("description")
            .and_then(Option::as_deref),
        Some("A feature (example)")
    );
    assert!(!metadata.metadata.contains_key("desc"));
    assert!(!metadata.metadata.contains_key("eg"));

    let parsed = parse_tf_file(dir.path().join("gloss.tf")).unwrap();
    let TfFeature::Node(feature) = parsed else {
        panic!("expected node feature");
    };
    assert_eq!(
        metadata_value(&feature, "description"),
        Some("A feature (example)")
    );

    let cache_path = dir.path().join("desc-eg.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    assert_eq!(
        mapped
            .metadata()
            .node_feature("gloss")
            .unwrap()
            .metadata_value("description"),
        Some("A feature (example)")
    );
}

#[test]
fn reports_invalid_tf_files() {
    assert!(
        parse_tf_file(repo_path(
            "libs/core/tests/fixtures/invalid/missing_header.tf"
        ))
        .is_err()
    );
    assert!(
        parse_tf_file(repo_path(
            "libs/core/tests/fixtures/invalid/corrupt_data.tf"
        ))
        .is_err()
    );
}

#[test]
fn treats_unknown_value_type_as_string_values() {
    let parsed = parse_tf_file(repo_path(
        "libs/core/tests/fixtures/invalid/bad_valueType.tf",
    ))
    .unwrap();
    let TfFeature::Node(feature) = parsed else {
        panic!("expected node feature");
    };
    assert_eq!(metadata_value(&feature, "valueType"), Some("float"));
    assert_eq!(feature.str_value(1), Some("1.5"));
    assert_eq!(feature.str_value(2), Some("2.5"));
}

#[test]
fn accepts_python_style_value_type_metadata_alias() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("otype.tf"),
        "@node\n@valueType=str\n\nword\nword\nphrase\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("oslots.tf"),
        "@edge\n@valueType=int\n\n3\t1-2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("legacy_number.tf"),
        "@node\n@value_type=int\n@description=legacy typed number\n\n7\n11\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("legacy_distance.tf"),
        "@edge\n@value_type=int\n@edgeValues\n@description=legacy typed edge\n\n1\t3\t9\n2\t3\t\n",
    )
    .unwrap();

    let parsed = parse_tf_file(dir.path().join("legacy_number.tf")).unwrap();
    let TfFeature::Node(parsed_feature) = parsed else {
        panic!("expected node feature");
    };
    assert_eq!(metadata_value(&parsed_feature, "value_type"), Some("int"));
    assert_eq!(parsed_feature.v(1), Some(&FeatureValue::Int(7)));
    assert_eq!(parsed_feature.v(2), Some(&FeatureValue::Int(11)));

    let cache_path = dir.path().join("legacy.cfr");
    compile_features(dir.path(), &cache_path, &[]).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let mapped_feature = mapped.mixed_node_feature("legacy_number").unwrap().unwrap();
    let entry = mapped.metadata().node_feature("legacy_number").unwrap();
    assert_eq!(entry.metadata_value("valueType"), Some("int"));
    assert_eq!(
        entry.metadata_value("description"),
        Some("legacy typed number")
    );
    assert_eq!(mapped_feature.v(1).unwrap(), Some(MappedNodeValue::Int(7)));
}
