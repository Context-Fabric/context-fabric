//! Node, edge and config feature views over a mapped corpus.

use std::collections::BTreeMap;
use std::fs;

mod common;

use common::*;
use context_fabric_core::*;

#[test]
fn string_pool_features_read_values() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(
        &source,
        &cache_path,
        &[
            "otype",
            "oslots",
            "word",
            "pos",
            "number",
            "score",
            "phrase_id",
            "parent",
            "relation",
            "distance",
        ],
    )
    .unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    assert_eq!(mapped.metadata().order_len, 8);
    assert_eq!(
        mapped.all_node_features(true),
        vec!["number", "otype", "phrase_id", "pos", "score", "word"]
    );
    assert_eq!(
        mapped.all_node_features(false),
        vec!["number", "phrase_id", "pos", "score", "word"]
    );
    assert_eq!(
        mapped.all_edge_features(true),
        vec!["distance", "oslots", "parent", "relation"]
    );
    assert_eq!(
        mapped.all_edge_features(false),
        vec!["distance", "parent", "relation"]
    );
    assert_eq!(mapped.order().unwrap(), vec![8, 6, 1, 2, 3, 7, 4, 5]);
    assert_eq!(mapped.rank().unwrap(), vec![2, 3, 4, 6, 7, 1, 5, 0]);
    assert_eq!(
        mapped.sort_key_tuple(&[1, 2, 3]).unwrap(),
        vec![Some(2), Some(3), Some(4)]
    );
    assert_eq!(
        mapped.otype_rank().unwrap(),
        BTreeMap::from([
            ("phrase".to_string(), 1),
            ("sentence".to_string(), 2),
            ("word".to_string(), 0),
        ])
    );
    assert_eq!(
        mapped.computed_feature("levels").unwrap(),
        Some(ComputedFeatureData::Levels(vec![
            ("sentence".to_string(), 5.0, 8, 8),
            ("phrase".to_string(), 2.5, 6, 7),
            ("word".to_string(), 1.0, 1, 5),
        ]))
    );
    assert_eq!(
        mapped.computed_feature("boundary").unwrap(),
        Some(ComputedFeatureData::Boundary(Boundary {
            first_slots: vec![vec![6, 8], vec![], vec![], vec![7], vec![]],
            last_slots: vec![vec![], vec![], vec![6], vec![], vec![8, 7]],
        }))
    );
    assert_eq!(mapped.computed_feature("missing").unwrap(), None);
    let mut mapped_nodes = vec![3, 1, 2];
    mapped.sort_nodes(&mut mapped_nodes).unwrap();
    assert_eq!(mapped_nodes, vec![1, 2, 3]);
    assert_eq!(mapped.sort_key(999).unwrap(), None);

    let word = mapped.string_pool_node_feature("word").unwrap().unwrap();
    let dynamic_word = mapped.node_feature("word").unwrap().unwrap();
    assert!(matches!(dynamic_word, MappedNodeFeatureView::StringPool(_)));
    assert_eq!(
        dynamic_word.metadata_value("description"),
        Some("word text")
    );
    assert_eq!(
        dynamic_word.v(1).unwrap(),
        Some(MappedNodeValue::Str("hello"))
    );
    assert_eq!(
        dynamic_word.s(MappedNodeValue::Str("hello")).unwrap(),
        vec![1]
    );
    assert_eq!(
        dynamic_word.items().unwrap()[0],
        (1, MappedNodeValue::Str("hello"))
    );
    assert_eq!(
        dynamic_word.s(MappedNodeValue::Int(1)).unwrap(),
        Vec::<u32>::new()
    );
    assert_eq!(word.row_count(), 5);
    assert_eq!(word.metadata_value("description"), Some("word text"));
    assert_eq!(word.str_value(1).unwrap(), Some("hello"));
    assert_eq!(word.v(1).unwrap(), Some(MappedNodeValue::Str("hello")));
    assert_eq!(word.str_value(5).unwrap(), Some("morning"));
    assert_eq!(word.str_value(8).unwrap(), None);
    assert_eq!(word.s("hello").unwrap(), vec![1]);
    let word_rows: Vec<_> = word
        .rows()
        .collect::<context_fabric_core::Result<_>>()
        .unwrap();
    assert_eq!(word_rows[0], (1, "hello"));
    assert_eq!(word_rows[4], (5, "morning"));
    let word_items = word.items().unwrap();
    assert_eq!(word_items[0], (1, MappedNodeValue::Str("hello")));
    assert_eq!(word_items[4], (5, MappedNodeValue::Str("morning")));

    let otype = mapped.string_pool_node_feature("otype").unwrap().unwrap();
    assert_eq!(otype.str_value(1).unwrap(), Some("word"));
    assert_eq!(otype.str_value(6).unwrap(), Some("phrase"));
    assert_eq!(otype.str_value(8).unwrap(), Some("sentence"));
    assert_eq!(otype.s("word").unwrap(), vec![1, 2, 3, 4, 5]);

    let number = mapped.mixed_node_feature("number").unwrap().unwrap();
    let dynamic_number = mapped.node_feature("number").unwrap().unwrap();
    assert!(matches!(dynamic_number, MappedNodeFeatureView::Mixed(_)));
    assert_eq!(
        dynamic_number.metadata_value("description"),
        Some("word count in phrase")
    );
    assert_eq!(dynamic_number.v(3).unwrap(), Some(MappedNodeValue::Int(3)));
    assert_eq!(
        dynamic_number.s(MappedNodeValue::Int(2)).unwrap(),
        vec![2, 5]
    );
    assert_eq!(number.v(3).unwrap(), Some(MappedNodeValue::Int(3)));
    assert_eq!(
        number.metadata_value("description"),
        Some("word count in phrase")
    );
    assert_eq!(number.s(MappedNodeValue::Int(2)).unwrap(), vec![2, 5]);
    assert_eq!(number.items().unwrap()[2], (3, MappedNodeValue::Int(3)));

    let score = mapped.mixed_node_feature("score").unwrap().unwrap();
    assert_eq!(score.v(2).unwrap(), Some(MappedNodeValue::Int(0)));
    assert_eq!(score.v(3).unwrap(), None);
    assert_eq!(score.s(MappedNodeValue::Int(0)).unwrap(), vec![2, 5]);

    assert!(
        mapped
            .string_pool_node_feature("missing")
            .unwrap()
            .is_none()
    );
    assert!(mapped.node_feature("missing").unwrap().is_none());
}

#[test]
fn config_features_read_metadata() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let otext = mapped.config_feature("otext").unwrap().unwrap();
    assert_eq!(
        otext.get("sectionTypes").unwrap().flatten(),
        Some("sentence,phrase")
    );
    assert_eq!(
        otext.metadata_value("sectionTypes").unwrap(),
        Some("sentence,phrase")
    );
    assert_eq!(
        otext.get("fmt:text-orig-full").unwrap().flatten(),
        Some("{word}")
    );
    assert_eq!(otext.get("missing").unwrap(), None);

    let metadata = otext.metadata().unwrap();
    assert_eq!(
        metadata.get("sectionFeatures").and_then(Option::as_deref),
        Some("sentence_id,phrase_id")
    );
}

#[test]
fn edge_features_read_targets() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &["otype", "oslots", "word", "pos"]).unwrap();

    let parsed = Corpus::load(&source).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let oslots = mapped.edge_feature("oslots").unwrap().unwrap();
    assert!(!oslots.has_edge_values());

    let phrase_slots: Vec<_> = oslots
        .targets(6)
        .unwrap()
        .unwrap()
        .collect::<context_fabric_core::Result<_>>()
        .unwrap();
    assert_eq!(phrase_slots, vec![1, 2, 3]);
    assert_eq!(oslots.s(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(oslots.f(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(oslots.forward(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(
        oslots.forward_with_values(6).unwrap(),
        vec![(1, None), (2, None), (3, None)]
    );
    assert_eq!(
        oslots.f_with_values(6).unwrap(),
        oslots.forward_with_values(6).unwrap()
    );

    let sentence_slots: Vec<_> = oslots
        .targets(8)
        .unwrap()
        .unwrap()
        .collect::<context_fabric_core::Result<_>>()
        .unwrap();
    assert_eq!(sentence_slots, vec![1, 2, 3, 4, 5]);
    assert_eq!(oslots.s(8).unwrap(), vec![1, 2, 3, 4, 5]);
    let mut embedders_for_slot_1 = vec![6, 8];
    parsed.sort_nodes(&mut embedders_for_slot_1);
    assert_eq!(oslots.t(1).unwrap(), embedders_for_slot_1);
    assert_eq!(oslots.backward(1).unwrap(), embedders_for_slot_1);
    assert_eq!(
        oslots.backward_with_values(1).unwrap(),
        embedders_for_slot_1
            .iter()
            .copied()
            .map(|node| (node, None))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        oslots.t_with_values(1).unwrap(),
        oslots.backward_with_values(1).unwrap()
    );
    assert_eq!(oslots.both(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(oslots.b(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(
        oslots.both_with_values(6).unwrap(),
        vec![(1, None), (2, None), (3, None)]
    );
    assert_eq!(
        oslots.b_with_values(6).unwrap(),
        oslots.both_with_values(6).unwrap()
    );
    assert_eq!(
        oslots.items().unwrap(),
        parsed.edge_feature("oslots").unwrap().items()
    );
    assert_eq!(
        oslots
            .items()
            .unwrap()
            .into_iter()
            .find(|(source, _)| *source == 8),
        Some((8, vec![1, 2, 3, 4, 5]))
    );

    assert!(oslots.targets(1).unwrap().is_none());
    assert!(mapped.edge_feature("missing").unwrap().is_none());
}

#[test]
fn edge_features_read_values() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(
        &source,
        &cache_path,
        &["otype", "oslots", "word", "relation", "distance"],
    )
    .unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();
    let relation = mapped.edge_feature("relation").unwrap().unwrap();
    assert!(relation.has_edge_values());
    assert_eq!(
        relation.edge_value(1, 6).unwrap(),
        Some(MappedNodeValue::Str("subject"))
    );
    assert_eq!(
        relation.edge_value(2, 6).unwrap(),
        Some(MappedNodeValue::Str("predicate"))
    );
    assert_eq!(relation.forward(1).unwrap(), vec![6]);
    assert_eq!(
        relation.forward_with_values(1).unwrap(),
        vec![(6, Some(MappedNodeValue::Str("subject")))]
    );
    assert_eq!(
        relation.f_with_values(1).unwrap(),
        relation.forward_with_values(1).unwrap()
    );
    assert_eq!(relation.backward(6).unwrap(), vec![1, 2, 3]);
    assert_eq!(
        relation.backward_with_values(6).unwrap(),
        vec![
            (1, Some(MappedNodeValue::Str("subject"))),
            (2, Some(MappedNodeValue::Str("predicate"))),
            (3, Some(MappedNodeValue::Str("object"))),
        ]
    );
    assert_eq!(
        relation.t_with_values(6).unwrap(),
        relation.backward_with_values(6).unwrap()
    );
    assert_eq!(relation.both(1).unwrap(), vec![6]);
    assert_eq!(relation.b(1).unwrap(), vec![6]);
    assert_eq!(
        relation.both_with_values(1).unwrap(),
        vec![(6, Some(MappedNodeValue::Str("subject")))]
    );
    assert_eq!(
        relation.b_with_values(1).unwrap(),
        relation.both_with_values(1).unwrap()
    );

    let distance = mapped.edge_feature("distance").unwrap().unwrap();
    assert!(distance.has_edge_values());
    assert_eq!(
        distance.edge_value(1, 2).unwrap(),
        Some(MappedNodeValue::Int(0))
    );
    assert_eq!(distance.edge_value(2, 3).unwrap(), None);
    assert_eq!(
        distance.forward_with_values(1).unwrap(),
        vec![
            (2, Some(MappedNodeValue::Int(0))),
            (3, Some(MappedNodeValue::Int(5)))
        ]
    );
    assert_eq!(
        distance.f_with_values(1).unwrap(),
        distance.forward_with_values(1).unwrap()
    );
    assert_eq!(
        distance.backward_with_values(2).unwrap(),
        vec![(1, Some(MappedNodeValue::Int(0)))]
    );
    assert_eq!(
        distance.t_with_values(2).unwrap(),
        distance.backward_with_values(2).unwrap()
    );
    assert_eq!(
        distance.backward_with_values(3).unwrap(),
        vec![(1, Some(MappedNodeValue::Int(5))), (2, None)]
    );
    assert_eq!(
        distance.t_with_values(3).unwrap(),
        distance.backward_with_values(3).unwrap()
    );
}

#[test]
fn compile_reload_cycle_preserves_string_edge_values() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &["otype", "oslots", "relation"]).unwrap();

    let fresh = MappedCompiledCorpus::open(&cache_path).unwrap();
    let cached = MappedCompiledCorpus::open(&cache_path).unwrap();
    let fresh_relation = fresh.edge_feature("relation").unwrap().unwrap();
    let cached_relation = cached.edge_feature("relation").unwrap().unwrap();

    assert_eq!(
        cached_relation.edge_value(1, 6).unwrap(),
        Some(MappedNodeValue::Str("subject"))
    );
    assert_eq!(
        cached_relation.edge_value(2, 6).unwrap(),
        Some(MappedNodeValue::Str("predicate"))
    );
    assert_eq!(
        cached_relation.edge_value(3, 6).unwrap(),
        Some(MappedNodeValue::Str("object"))
    );

    for source_node in 1..=5 {
        assert_eq!(
            fresh_relation.forward_with_values(source_node).unwrap(),
            cached_relation.forward_with_values(source_node).unwrap(),
            "string edge values differ after cache reopen for source node {source_node}"
        );
    }
}

#[test]
fn frequency_lists_count_values_with_type_filters() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();

    assert_eq!(
        mapped.node_frequency_list("pos", Some(&["word"])).unwrap(),
        vec![
            (FeatureValue::string("adjective"), 2),
            (FeatureValue::string("noun"), 2),
            (FeatureValue::string("interjection"), 1),
        ]
    );
    assert!(
        mapped
            .node_frequency_list("pos", Some(&["phrase"]))
            .unwrap()
            .is_empty()
    );
    assert!(mapped.node_frequency_list("missing", None).is_err());

    assert_eq!(
        mapped.edge_frequency_list("parent", None, None).unwrap(),
        EdgeFrequency::Count(7)
    );
    assert_eq!(
        mapped
            .edge_frequency_list("parent", Some(&["word"]), Some(&["phrase"]))
            .unwrap(),
        EdgeFrequency::Count(5)
    );
    assert_eq!(
        mapped
            .edge_frequency_list("parent", Some(&["phrase"]), Some(&["sentence"]))
            .unwrap(),
        EdgeFrequency::Count(2)
    );
    assert_eq!(
        mapped.edge_frequency_list("distance", None, None).unwrap(),
        EdgeFrequency::Values(vec![
            (Some(FeatureValue::Int(0)), 3),
            (None, 3),
            (Some(FeatureValue::Int(5)), 1),
            (Some(FeatureValue::Int(10)), 1),
        ])
    );
    assert!(mapped.edge_frequency_list("missing", None, None).is_err());
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

    let tag = mapped.string_pool_node_feature("tag").unwrap().unwrap();
    let mapped_nodes = tag.s("shared").unwrap();
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
    assert_eq!(
        distance.edge_value(1, 2).unwrap(),
        Some(MappedNodeValue::Int(0))
    );
    assert_eq!(
        distance.edge_value(3, 4).unwrap(),
        Some(MappedNodeValue::Int(0))
    );
    assert_eq!(
        distance.edge_value(6, 7).unwrap(),
        Some(MappedNodeValue::Int(0))
    );
    // Non-zero values.
    assert_eq!(
        distance.edge_value(1, 3).unwrap(),
        Some(MappedNodeValue::Int(5))
    );
    assert_eq!(
        distance.edge_value(4, 5).unwrap(),
        Some(MappedNodeValue::Int(10))
    );
    // Edge present but value absent -> None.
    assert!(distance.forward(2).unwrap().contains(&3));
    assert_eq!(distance.edge_value(2, 3).unwrap(), None);
    // No such edge -> None.
    assert_eq!(distance.edge_value(1, 4).unwrap(), None);

    // Binary search yields the same answers when queried repeatedly (index is
    // cached after first build).
    assert_eq!(
        distance.edge_value(1, 2).unwrap(),
        Some(MappedNodeValue::Int(0))
    );
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

#[test]
fn self_referential_empty_and_bidirectional_valued_edges() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("edges");
    write_files(
        &source,
        &[
            ("otype.tf", "@node\n@valueType=str\n\nword\nword\nword\n"),
            ("oslots.tf", "@edge\n@valueType=int\n\n"),
            (
                "link.tf",
                "@edge\n@edgeValues\n@valueType=str\n\n1\t2\tout\n2\t1\tin\n3\t3\tself\n",
            ),
            ("empty.tf", "@edge\n@valueType=int\n\n"),
        ],
    );
    let cache = temp.path().join("edges.cfr");
    compile_features(&source, &cache, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache).unwrap();

    let link = mapped.edge_feature("link").unwrap().unwrap();
    assert_eq!(link.f(3).unwrap(), vec![3]);
    assert_eq!(link.t(3).unwrap(), vec![3]);
    assert_eq!(link.b(3).unwrap(), vec![3]);
    // A pair linked both ways reports the outgoing value.
    assert_eq!(
        link.both_with_values(1).unwrap(),
        vec![(2, Some(MappedNodeValue::Str("out")))]
    );
    assert_eq!(
        link.both_with_values(2).unwrap(),
        vec![(1, Some(MappedNodeValue::Str("in")))]
    );

    let empty = mapped.edge_feature("empty").unwrap().unwrap();
    assert!(empty.f(1).unwrap().is_empty());
    assert!(empty.t(1).unwrap().is_empty());
    assert!(empty.b(1).unwrap().is_empty());
    assert!(empty.items().unwrap().is_empty());
}
