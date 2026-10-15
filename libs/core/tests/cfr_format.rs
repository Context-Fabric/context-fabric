//! Robustness of the compiled `.cfr` cache reader and writer against
//! corrupted, truncated, and concurrently replaced files.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod common;

use common::*;
use context_fabric_core::*;

const MAGIC: &[u8; 8] = b"CFRUST04";

fn write_bytes(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, bytes).expect("write crafted cache");
    path
}

fn header_with_counts(counts: &[u32]) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    for count in counts {
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    bytes
}

/// A corrupted count field must fail fast. Before capacities were bounded by
/// the remaining bytes, a `u32::MAX` count pre-allocated (and for hash maps,
/// initialized) gigabytes of memory, stalling for over a minute.
fn assert_fails_fast(path: &Path) -> String {
    let started = Instant::now();
    let error = inspect_compiled(path).expect_err("corrupted cache must fail");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "rejecting {} took {:?}",
        path.display(),
        started.elapsed()
    );
    error.to_string()
}

#[test]
fn huge_declared_config_feature_count_is_rejected_without_allocating() {
    // node count, edge count, order len, rank len, config feature count.
    let dir = tempfile::tempdir().unwrap();
    let path = write_bytes(
        &dir,
        "configs.cfr",
        &header_with_counts(&[0, 0, 0, 0, u32::MAX]),
    );
    let error = assert_fails_fast(&path);
    assert!(error.contains("unexpected end"), "{error}");
}

#[test]
fn huge_declared_edge_value_section_count_is_rejected_without_allocating() {
    // node, edge, order, rank, config counts all zero; empty feature metadata
    // section (two zero maps); edge-values section declaring u32::MAX features.
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = header_with_counts(&[0, 0, 0, 0, 0]);
    bytes.extend_from_slice(b"CFRMETA1");
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(b"CFREDGE1");
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    let path = write_bytes(&dir, "edges.cfr", &bytes);
    let error = assert_fails_fast(&path);
    assert!(error.contains("unexpected end"), "{error}");
}

#[test]
fn empty_and_foreign_files_fail_with_a_clear_error() {
    let dir = tempfile::tempdir().unwrap();
    for (name, bytes) in [
        ("empty.cfr", Vec::new()),
        ("foreign.cfr", b"PK\x03\x04 not a cache".to_vec()),
        ("magic_only.cfr", MAGIC.to_vec()),
    ] {
        let path = write_bytes(&dir, name, &bytes);
        let error = MappedCompiledCorpus::open(&path).expect_err(name);
        assert!(
            error.to_string().contains(path.to_str().unwrap()),
            "error for {name} should name the file: {error}"
        );
    }
}

#[test]
fn every_truncation_of_a_valid_cache_fails_cleanly_or_opens() {
    let dir = tempfile::tempdir().unwrap();
    let full = dir.path().join("full.cfr");
    compile_features(mini_corpus_dir(), &full, &[]).unwrap();
    let bytes = fs::read(&full).unwrap();
    for len in 0..bytes.len() {
        let path = write_bytes(&dir, "truncated.cfr", &bytes[..len]);
        // Either outcome is acceptable; panicking or aborting is not.
        let _ = MappedCompiledCorpus::open(&path);
    }
}

#[test]
fn recompiling_over_an_open_cache_leaves_the_open_mapping_intact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corpus.cfr");
    compile_features(mini_corpus_dir(), &path, &[]).unwrap();
    let open = MappedCompiledCorpus::open(&path).unwrap();
    let expected = open
        .string_pool_node_feature("word")
        .unwrap()
        .unwrap()
        .str_value(1)
        .unwrap()
        .map(str::to_string);

    // Another process (or a stale-cache rebuild) replaces the file with a
    // much smaller cache. Truncating in place would shrink the file under the
    // live mapping and fault (SIGBUS) on the next read.
    compile_features(mini_corpus_dir(), &path, &["otype", "oslots"]).unwrap();
    assert!(fs::metadata(&path).unwrap().len() < open.metadata().byte_len as u64);

    let after = open
        .string_pool_node_feature("word")
        .unwrap()
        .unwrap()
        .str_value(1)
        .unwrap()
        .map(str::to_string);
    assert_eq!(after, expected);
}

#[test]
fn compiling_leaves_no_partial_files_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corpus.cfr");
    compile_features(mini_corpus_dir(), &path, &[]).unwrap();
    let names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, vec!["corpus.cfr".to_string()]);
}

#[test]
fn failed_compile_keeps_the_previous_cache() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corpus.cfr");
    compile_features(mini_corpus_dir(), &path, &[]).unwrap();
    let before = fs::read(&path).unwrap();
    let missing = dir.path().join("no_such_corpus");
    assert!(compile_features(&missing, &path, &[]).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn compiled_cache_is_readable_by_other_users() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corpus.cfr");
    compile_features(mini_corpus_dir(), &path, &[]).unwrap();
    let mode = fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o044, 0o044, "mode {mode:o}");
}

#[test]
fn compiled_cache_can_be_inspected() {
    let source = repo_path("libs/core/tests/fixtures/mini_corpus");
    let temp_dir = tempfile::tempdir().unwrap();
    let cache_path = temp_dir.path().join("mini.cfr");

    compile_features(&source, &cache_path, &[]).unwrap();

    let metadata = inspect_compiled(&cache_path).unwrap();
    assert!(metadata.byte_len > 0);
    assert_eq!(metadata.node_features.len(), 7);
    assert_eq!(metadata.edge_features.len(), 4);
    assert_eq!(metadata.config_features.len(), 1);
    assert_eq!(metadata.order_len, 8);
    assert_eq!(metadata.rank_len, 8);

    let otype = metadata.node_feature("otype").unwrap();
    assert_eq!(otype.encoding, NodeFeatureEncoding::StringPool);
    assert_eq!(otype.row_count, 8);
    assert_eq!(otype.string_pool_count, Some(3));
    assert!(otype.payload_start < otype.payload_end);
    assert_eq!(
        metadata
            .node_feature("word")
            .unwrap()
            .metadata
            .get("description")
            .and_then(Option::as_deref),
        Some("word text")
    );

    let number = metadata.node_feature("number").unwrap();
    assert_eq!(number.encoding, NodeFeatureEncoding::Mixed);
    assert_eq!(number.row_count, 5);

    let oslots = metadata.edge_feature("oslots").unwrap();
    assert_eq!(oslots.row_count, 3);
    assert!(oslots.payload_start < oslots.payload_end);
    assert_eq!(
        metadata
            .edge_feature("distance")
            .unwrap()
            .metadata
            .get("valueType")
            .and_then(Option::as_deref),
        Some("int")
    );

    let otext = metadata.config_feature("otext").unwrap();
    assert_eq!(otext.metadata_count, 5);
    assert!(otext.payload_start < otext.payload_end);
}

#[test]
fn compiled_cache_stores_the_structure() {
    let temp_dir = tempfile::tempdir().unwrap();
    let source = structured_mini_corpus(&temp_dir);
    let cache_path = temp_dir.path().join("structured.cfr");
    compile_features(&source, &cache_path, &[]).unwrap();
    let mapped = MappedCompiledCorpus::open(&cache_path).unwrap();

    let heading_7 = vec![
        StructureHeading {
            node_type: "sentence".to_string(),
            heading: Some(FeatureValue::string("S1")),
        },
        StructureHeading {
            node_type: "phrase".to_string(),
            heading: Some(FeatureValue::Int(2)),
        },
    ];
    assert!(mapped.metadata().structure_start.is_some());
    assert_eq!(mapped.top().unwrap(), Some(vec![8]));
    assert_eq!(
        mapped.heading_from_node(7).unwrap(),
        Some(heading_7.clone())
    );
    assert_eq!(mapped.node_from_heading(&heading_7).unwrap(), Some(7));
    assert_eq!(
        mapped.structure(Some(8)).unwrap(),
        Some(StructureTree::Node {
            node: 8,
            children: vec![
                StructureTree::Node {
                    node: 6,
                    children: vec![]
                },
                StructureTree::Node {
                    node: 7,
                    children: vec![]
                },
            ],
        })
    );
    assert_eq!(
        mapped.structure_pretty(Some(8), false).unwrap().as_deref(),
        Some("  sentence:S1\n      phrase:1\n      phrase:2")
    );
    assert_eq!(
        mapped.structure_pretty(Some(7), true).unwrap().as_deref(),
        Some("  sentence:S1-phrase:2")
    );
}

/// The index sections of the three-level corpus (words 1-6, verses 7 and 8,
/// chapter 9 and book 10 over the same six slots) read back row by row.
#[test]
fn compiled_cache_stores_the_index_sections() {
    let temp = tempfile::tempdir().unwrap();
    let mapped = mapped_three_level_corpus(&temp);

    let lev_up: [&[u32]; 10] = [
        &[7, 9, 10],
        &[7, 9, 10],
        &[7, 9, 10],
        &[8, 9, 10],
        &[8, 9, 10],
        &[8, 9, 10],
        &[9, 10],
        &[9, 10],
        &[10],
        &[9],
    ];
    let lev_down: [&[u32]; 10] = [
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[],
        &[10, 7, 8],
        &[9, 7, 8],
    ];
    for node in 1..=10 {
        let index = node as usize - 1;
        assert_eq!(
            mapped.lev_up_row(node).unwrap().unwrap(),
            lev_up[index],
            "levUp {node}"
        );
        assert_eq!(
            mapped.lev_down_row(node).unwrap().unwrap(),
            lev_down[index],
            "levDown {node}"
        );
    }
    assert_eq!(mapped.lev_up_row(11).unwrap(), None);

    let first: [&[u32]; 6] = [&[7, 9, 10], &[], &[], &[8], &[], &[]];
    let last: [&[u32]; 6] = [&[], &[], &[7], &[], &[], &[10, 9, 8]];
    for slot in 1..=6 {
        let index = slot as usize - 1;
        assert_eq!(mapped.boundary_first(slot).unwrap().unwrap(), first[index]);
        assert_eq!(mapped.boundary_last(slot).unwrap().unwrap(), last[index]);
    }

    let sections = mapped.sections_data().unwrap().expect("sections present");
    assert_eq!(sections.sec1[&10]["1"], 9);
    assert_eq!(sections.sec2[&10]["1"]["2"], 8);
    assert_eq!(sections.seq_from_node[&8], vec![1, 1, 2]);
    assert_eq!(sections.node_from_seq[&vec![1, 1, 1]], 7);

    match mapped.computed_feature("levDown").unwrap() {
        Some(ComputedFeatureData::LevDown(rows)) => {
            assert_eq!(rows, vec![vec![], vec![], vec![10, 7, 8], vec![9, 7, 8]]);
        }
        other => panic!("expected LevDown, got {other:?}"),
    }
}

/// A cache in another `.cfr` format version (every pre-release format had the
/// `CFRUST02` header) is rejected with a hint to recompile, not read.
#[test]
fn caches_of_another_format_version_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("three_level");
    write_three_level_corpus(&source);
    let cache = temp.path().join("three_level.cfr");
    compile_features(&source, &cache, &[]).unwrap();

    let mut bytes = fs::read(&cache).unwrap();
    assert_eq!(&bytes[..8], b"CFRUST04");
    bytes[..8].copy_from_slice(b"CFRUST02");
    fs::write(&cache, &bytes).unwrap();

    let error = MappedCompiledCorpus::open(&cache).unwrap_err();
    assert!(
        matches!(error, CfError::IncompatibleCache { ref found, .. } if found == "02"),
        "{error:?}"
    );
    let message = error.to_string();
    assert!(
        message.contains("format 02") && message.contains("recompile"),
        "{message}"
    );
}

/// Every index section is mandatory: a cache cut short before them is corrupt.
#[test]
fn caches_missing_an_index_section_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("three_level");
    write_three_level_corpus(&source);
    let cache = temp.path().join("three_level.cfr");
    compile_features(&source, &cache, &[]).unwrap();

    let bytes = fs::read(&cache).unwrap();
    let lev_up = bytes
        .windows(8)
        .position(|window| window == b"CFRLEVU1")
        .expect("levUp section present");
    fs::write(&cache, &bytes[..lev_up]).unwrap();
    assert!(matches!(
        MappedCompiledCorpus::open(&cache),
        Err(CfError::Parse { .. })
    ));
}
