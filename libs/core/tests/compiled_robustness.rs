//! Robustness of the compiled `.cfr` cache reader and writer against
//! corrupted, truncated, and concurrently replaced files.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use context_fabric_core::compiled::{MappedCompiledCorpus, compile_features, inspect_compiled};

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

fn mini_corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini_corpus")
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
    compile_features(mini_corpus(), &full, &[]).unwrap();
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
    compile_features(mini_corpus(), &path, &[]).unwrap();
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
    compile_features(mini_corpus(), &path, &["otype", "oslots"]).unwrap();
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
    compile_features(mini_corpus(), &path, &[]).unwrap();
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
    compile_features(mini_corpus(), &path, &[]).unwrap();
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
    compile_features(mini_corpus(), &path, &[]).unwrap();
    let mode = fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o044, 0o044, "mode {mode:o}");
}
