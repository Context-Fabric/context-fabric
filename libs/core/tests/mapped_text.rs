//! Text rendering from the `otext` formats of a mapped corpus.

use std::sync::Arc;

mod common;

use common::*;
use context_fabric_core::*;

#[test]
fn renders_otext_formats_and_splits_format_names() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mapped = Arc::new(mapped_mini_corpus(&temp_dir));
    let renderer = TextRenderer::new(&mapped).unwrap();
    let text = |node, format: Option<&str>| {
        renderer
            .text(node, &TextOptions::new(format, None))
            .unwrap()
    };

    assert_eq!(
        renderer.split_format("phrase#{word}"),
        ("phrase".to_string(), "{word}".to_string())
    );
    assert_eq!(
        renderer.split_format("unknown#{word}"),
        ("word".to_string(), "unknown#{word}".to_string())
    );
    assert_eq!(
        renderer.split_default_format("phrase-default"),
        Some("phrase".to_string())
    );
    assert_eq!(renderer.split_default_format("unknown-default"), None);
    assert_eq!(renderer.split_default_format("phrase-text"), None);
    assert_eq!(
        renderer.formats().unwrap(),
        std::collections::BTreeMap::from([("text-orig-full".to_string(), "word".to_string())])
    );

    assert_eq!(text(1, None), "hello");
    assert_eq!(text(1, Some("fmt:text-orig-full")), "hello");
    assert_eq!(text(6, None), "hellobeautifulworld");
    assert_eq!(text(8, None), "hellobeautifulworldgoodmorning");
    assert_eq!(
        renderer.rendered_nodes(6, &TextOptions::default()).unwrap(),
        [1, 2, 3]
    );
    assert_eq!(text(1, Some("missing-format")), "");
}

#[test]
fn reports_characters_per_format() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&temp_dir);
    let characters = mapped.format_characters().unwrap();
    let counts = &characters["text-orig-full"];
    // hello beautiful world good morning
    assert_eq!(counts[&'o'], 5);
    assert_eq!(counts[&'h'], 1);
    assert!(!counts.contains_key(&' '));
}
