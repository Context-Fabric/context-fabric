//! Text rendering from the `otext` formats of a mapped corpus.

mod common;

use common::*;
use context_fabric_core::*;

#[test]
fn renders_otext_formats_and_splits_format_names() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mapped = mapped_mini_corpus(&temp_dir);
    let text = MappedText::new(&mapped).unwrap();

    assert_eq!(
        text.split_format("phrase#{word}").unwrap(),
        ("phrase".to_string(), "{word}".to_string())
    );
    assert_eq!(
        text.split_format("unknown#{word}").unwrap(),
        ("word".to_string(), "unknown#{word}".to_string())
    );
    assert_eq!(
        text.split_format("{word}").unwrap(),
        ("word".to_string(), "{word}".to_string())
    );
    assert_eq!(
        text.split_default_format("phrase-default").unwrap(),
        Some("phrase".to_string())
    );
    assert_eq!(text.split_default_format("unknown-default").unwrap(), None);
    assert_eq!(text.split_default_format("phrase-text").unwrap(), None);

    assert_eq!(text.text(1, None).unwrap(), "hello");
    assert_eq!(
        text.text(1, Some("fmt:text-orig-full")).unwrap(),
        text.text(1, Some("text-orig-full")).unwrap()
    );
    assert_eq!(text.text(6, None).unwrap(), "hellobeautifulworld");
    assert_eq!(
        text.text(8, None).unwrap(),
        "hellobeautifulworldgoodmorning"
    );
    assert_eq!(
        text.text_nodes(&[3, 1, 2], None).unwrap(),
        "worldhellobeautiful"
    );
    assert_eq!(text.text(1, Some("missing-format")).unwrap(), "");
}
