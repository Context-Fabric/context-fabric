//! A corpus in a read-only directory still loads: its compiled cache goes to
//! the user cache directory. Kept in its own test binary because it points
//! `CFABRIC_CACHE` (process-global) at a temporary directory.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use context_fabric_core::Fabric;

struct RestoreWritable<'a>(&'a Path);

impl Drop for RestoreWritable<'_> {
    fn drop(&mut self) {
        let _ = fs::set_permissions(self.0, fs::Permissions::from_mode(0o755));
    }
}

#[test]
fn read_only_corpus_compiles_into_the_user_cache_directory() {
    let dir = tempfile::tempdir().unwrap();
    let user_cache = dir.path().join("user-cache");
    // SAFETY: this test binary has a single test, so no other thread reads the
    // environment concurrently.
    unsafe { std::env::set_var("CFABRIC_CACHE", &user_cache) };

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini_corpus");
    let corpus = dir.path().join("mini");
    fs::create_dir_all(&corpus).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "tf") {
            fs::copy(&path, corpus.join(path.file_name().unwrap())).unwrap();
        }
    }
    fs::set_permissions(&corpus, fs::Permissions::from_mode(0o555)).unwrap();
    let _restore = RestoreWritable(&corpus);
    if fs::File::create(corpus.join("probe")).is_ok() {
        eprintln!("skipping: running with privileges that ignore directory permissions");
        return;
    }

    let fabric = Fabric::new(&corpus);
    let corpus_api = fabric.load_all().expect("read-only corpus must still load");
    assert_eq!(corpus_api.max_node(), 8);
    assert!(!corpus.join(".cfr").exists());
    assert!(corpus_api.path().starts_with(&user_cache));

    // A second load reuses the fallback cache.
    let reloaded = fabric.load_all().unwrap();
    assert_eq!(reloaded.path(), corpus_api.path());
}
