//! Freshness and placement of the compiled cache that `Fabric::load_all`
//! builds beside a corpus's `.tf` sources.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use context_fabric_core::{Fabric, MappedCompiledCorpus};

fn copy_mini_corpus(into: &Path) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mini_corpus");
    let corpus = into.join("mini");
    fs::create_dir_all(&corpus).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|extension| extension == "tf") {
            fs::copy(&path, corpus.join(path.file_name().unwrap())).unwrap();
        }
    }
    corpus
}

fn node_features(fabric: &Fabric) -> Vec<String> {
    fabric.load_all().unwrap().all_node_features(true)
}

#[test]
fn deleting_a_feature_file_invalidates_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = copy_mini_corpus(dir.path());
    let fabric = Fabric::new(&corpus);
    assert!(node_features(&fabric).contains(&"score".to_string()));

    fs::remove_file(corpus.join("score.tf")).unwrap();
    assert!(!node_features(&fabric).contains(&"score".to_string()));
}

#[test]
fn replacing_a_feature_file_with_an_older_one_invalidates_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = copy_mini_corpus(dir.path());
    let fabric = Fabric::new(&corpus);
    let word = |fabric: &Fabric| {
        fabric
            .load_all()
            .unwrap()
            .string_pool_node_feature("word")
            .unwrap()
            .unwrap()
            .str_value(1)
            .unwrap()
            .map(str::to_string)
    };
    assert_eq!(word(&fabric).as_deref(), Some("hello"));

    // Unpacking a newer corpus release preserves archive mtimes, which can be
    // older than the cache.
    let word_path = corpus.join("word.tf");
    let replaced = fs::read_to_string(&word_path)
        .unwrap()
        .replace("hello", "shalom");
    fs::write(&word_path, replaced).unwrap();
    let past = SystemTime::now() - Duration::from_secs(365 * 24 * 3600);
    fs::File::options()
        .write(true)
        .open(&word_path)
        .unwrap()
        .set_modified(past)
        .unwrap();
    assert_eq!(word(&fabric).as_deref(), Some("shalom"));
}

#[test]
fn a_composed_corpus_cache_never_stands_in_for_one_of_its_locations() {
    let dir = tempfile::tempdir().unwrap();
    let base = copy_mini_corpus(dir.path());
    let extra = dir.path().join("extra");
    fs::create_dir_all(&extra).unwrap();
    fs::write(
        extra.join("gloss.tf"),
        "@node\n@valueType=str\n@writtenBy=test\n\n1\tgreeting\n",
    )
    .unwrap();

    let composed = Fabric::from_locations([&base, &extra]);
    assert!(node_features(&composed).contains(&"gloss".to_string()));

    // `extra` alone has no warp features; reusing the composed cache stored in
    // `extra` would silently hand back the base corpus.
    let alone = Fabric::new(&extra);
    assert!(alone.load_all().is_err());
    assert_ne!(
        composed.cfr_cache_path().unwrap(),
        alone.cfr_cache_path().unwrap()
    );
}

#[test]
fn an_unchanged_corpus_reuses_its_cache() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = copy_mini_corpus(dir.path());
    let fabric = Fabric::new(&corpus);
    fabric.load_all().unwrap();
    let cache = fabric.cfr_cache_path().unwrap();
    let first = fs::metadata(&cache).unwrap().modified().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    fabric.load_all().unwrap();
    assert_eq!(fs::metadata(&cache).unwrap().modified().unwrap(), first);
}

#[test]
fn a_corpus_mapped_from_a_stale_cache_keeps_working_after_the_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    let corpus = copy_mini_corpus(dir.path());
    let fabric = Fabric::new(&corpus);
    let stale = fabric.load_all().unwrap();
    let word = |corpus: &MappedCompiledCorpus| {
        corpus
            .string_pool_node_feature("word")
            .unwrap()
            .unwrap()
            .str_value(1)
            .unwrap()
            .map(str::to_string)
    };

    let word_path = corpus.join("word.tf");
    let replaced = fs::read_to_string(&word_path)
        .unwrap()
        .replace("hello", "shalom");
    fs::write(&word_path, replaced).unwrap();
    let fresh = fabric.load_all().unwrap();

    assert_eq!(word(&fresh).as_deref(), Some("shalom"));
    assert_eq!(word(&stale).as_deref(), Some("hello"));
}

#[cfg(unix)]
#[test]
fn the_cache_is_readable_by_other_users() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let corpus = copy_mini_corpus(dir.path());
    let fabric = Fabric::new(&corpus);
    fabric.load_all().unwrap();
    let cache = fabric.cfr_cache_path().unwrap();
    let mode = fs::metadata(&cache).unwrap().permissions().mode();
    assert_eq!(mode & 0o044, 0o044, "{} mode {mode:o}", cache.display());
}

/// A default cache in another `.cfr` format version is stale: `loadAll`
/// rebuilds it in the current format instead of failing to open it.
#[test]
fn default_caches_of_another_format_version_are_rebuilt() {
    let temp_dir = tempfile::tempdir().unwrap();
    let corpus_dir = copy_mini_corpus(temp_dir.path());
    let fabric = Fabric::new(&corpus_dir);
    let cache_path = fabric.cfr_cache_path().unwrap();
    drop(fabric.load_all().unwrap());

    let mut bytes = fs::read(&cache_path).unwrap();
    bytes[..8].copy_from_slice(b"CFRUST02");
    fs::write(&cache_path, &bytes).unwrap();
    assert!(MappedCompiledCorpus::open(&cache_path).is_err());

    let rebuilt = fabric.load_all().unwrap();
    assert_eq!(rebuilt.max_node(), 8);
    assert_eq!(&fs::read(&cache_path).unwrap()[..8], b"CFRUST04");
}
