//! Location and freshness of the compiled `.cfr` cache that `Fabric.load`
//! builds from a corpus's `.tf` sources.
//!
//! A cache is named after the [`SourceManifest`] fingerprint of the sources
//! it was compiled from: the ordered module directories, every `.tf` file's
//! name, size, and modification time, and the format and engine versions.
//! Editing, adding, deleting, or replacing a feature file (even with an older
//! one) changes the fingerprint and so the file name, and a cache compiled
//! from one location set never stands in for another.
//!
//! Caches are therefore never replaced in place. A stale cache stays where it
//! is, so processes that memory-mapped it keep working, while the fresh one is
//! published beside it under its own name; stale caches are pruned once they
//! can be deleted. Processes that compile the same sources at once all publish
//! to the same name without replacing: the first one wins and the others
//! discard their copy and map the winner's. This works the same on Windows,
//! where a file that another process has open or mapped cannot be replaced or
//! deleted.
//!
//! Caches live inside the last module directory
//! (`<module>/.cfr/<version>/corpus.<fingerprint>.cfr`) so they travel with
//! the corpus. When that directory is not writable, the cache is placed under
//! the user cache directory instead
//! (`<cache dir>/cfr/<location key>/<version>/`).

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::atomic_file::{Publication, STAGED_SUFFIX, StagedFile};
use crate::compile::Corpus;
use crate::config::{CFR_VERSION, VERSION};
use crate::error::{CfError, Result};
use crate::format::{inspect_compiled, write_compiled_to};

const CACHE_DIR_NAME: &str = ".cfr";
const CACHE_FILE_STEM: &str = "corpus";
const CACHE_EXTENSION: &str = "cfr";
/// Number of hexadecimal digits in a [`SourceManifest::fingerprint`].
const FINGERPRINT_DIGITS: usize = 16;
/// Staged caches left untouched this long belong to compilers that died.
const ABANDONED_STAGED_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// One `.tf` source file as seen when a cache is compiled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SourceFile {
    /// Index of the module directory in [`SourceManifest::module_paths`].
    module: usize,
    name: String,
    len: u64,
    modified_ns: u128,
}

/// Snapshot of the `.tf` sources a compiled cache is built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceManifest {
    cfr_version: String,
    /// Engine release that writes the cache: a new release recompiles once, so
    /// a layout change can never be read with a stale reader.
    engine_version: String,
    module_paths: Vec<PathBuf>,
    files: Vec<SourceFile>,
}

impl SourceManifest {
    /// Scans `module_paths` (in load order) for `.tf` files.
    pub fn scan(module_paths: &[PathBuf]) -> Result<Self> {
        let module_paths = canonical_paths(module_paths);
        let mut files = Vec::new();
        for (module, module_path) in module_paths.iter().enumerate() {
            let entries = fs::read_dir(module_path).map_err(|source| CfError::Io {
                path: module_path.clone(),
                source,
            })?;
            for entry in entries {
                let entry = entry.map_err(|source| CfError::Io {
                    path: module_path.clone(),
                    source,
                })?;
                let path = entry.path();
                if path.extension().and_then(|extension| extension.to_str()) != Some("tf") {
                    continue;
                }
                let metadata = entry.metadata().map_err(|source| CfError::Io {
                    path: path.clone(),
                    source,
                })?;
                if !metadata.is_file() {
                    continue;
                }
                let modified_ns = metadata
                    .modified()
                    .ok()
                    .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |since_epoch| since_epoch.as_nanos());
                files.push(SourceFile {
                    module,
                    name: entry.file_name().to_string_lossy().into_owned(),
                    len: metadata.len(),
                    modified_ns,
                });
            }
        }
        files.sort_by(|left, right| (left.module, &left.name).cmp(&(right.module, &right.name)));
        Ok(Self {
            cfr_version: CFR_VERSION.to_string(),
            engine_version: VERSION.to_string(),
            module_paths,
            files,
        })
    }

    /// Hexadecimal digest of the whole manifest, naming the cache compiled
    /// from these sources.
    pub fn fingerprint(&self) -> Result<String> {
        let digest = fnv1a_64(&serde_json::to_vec(self)?);
        Ok(format!("{digest:0width$x}", width = FINGERPRINT_DIGITS))
    }
}

/// The compiled caches of one ordered set of module directories, and the two
/// directories they may live in, in order of preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusCache {
    module_paths: Vec<PathBuf>,
    /// Directory inside the last module directory, travelling with the corpus.
    in_corpus: PathBuf,
    /// Directory under the user cache directory, for read-only corpus
    /// directories.
    user_cache: PathBuf,
    /// File-name stem of every cache of these module paths: one location
    /// keeps the conventional stem, a composition of several gets its own so
    /// its caches never mix with the single-location ones.
    stem: String,
}

impl CorpusCache {
    /// The caches of `module_paths`, with the fallback directory under
    /// [`user_cache_dir`].
    pub fn new(module_paths: Vec<PathBuf>) -> Self {
        Self::with_user_cache(module_paths, &user_cache_dir())
    }

    /// The caches of `module_paths`, with the fallback directory under
    /// `user_cache_root`.
    pub fn with_user_cache(module_paths: Vec<PathBuf>, user_cache_root: &Path) -> Self {
        // The key is derived from canonical paths so `./corpus` and its
        // absolute spelling share a cache; the in-corpus path keeps the
        // caller's spelling.
        let key = location_key(&canonical_paths(&module_paths));
        let last_module = module_paths
            .last()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("."));
        let stem = if module_paths.len() > 1 {
            format!("{CACHE_FILE_STEM}-{key}")
        } else {
            CACHE_FILE_STEM.to_string()
        };
        Self {
            in_corpus: last_module.join(CACHE_DIR_NAME).join(CFR_VERSION),
            user_cache: user_cache_root.join("cfr").join(key).join(CFR_VERSION),
            module_paths,
            stem,
        }
    }

    /// Path inside the corpus of the cache for the current sources (the
    /// preferred location; see [`Self::ensure_compiled`] for the fallback).
    pub fn in_corpus_path(&self) -> Result<PathBuf> {
        Ok(self.cache_file(&self.in_corpus, &self.fingerprint()?))
    }

    /// Returns the path of a valid cache for the current sources, compiling
    /// the corpus that `build` returns when no location holds one.
    ///
    /// A location is skipped in favour of the next when its directory cannot
    /// be written (permission denied or read-only file system) or when the
    /// cache's name there is held by a file that cannot be replaced.
    pub fn ensure_compiled(&self, build: impl Fn() -> Result<Corpus>) -> Result<PathBuf> {
        let fingerprint = self.fingerprint()?;
        let candidates = self
            .directories()
            .map(|directory| self.cache_file(directory, &fingerprint));
        if let Some(valid) = candidates.iter().find(|candidate| is_valid(candidate)) {
            return Ok(valid.clone());
        }
        let mut corpus = None;
        let mut first_error = None;
        for candidate in candidates {
            match compile_into(&candidate, &mut corpus, &build) {
                Ok(()) => {
                    self.prune_around(&candidate);
                    return Ok(candidate);
                }
                Err(error) if is_location_unusable(&error) => {
                    first_error.get_or_insert(error);
                }
                Err(error) => return Err(error),
            }
        }
        Err(first_error.expect("at least one cache location was attempted"))
    }

    /// Deletes every cache of these module paths from both directories.
    ///
    /// Caches being written by a running compile are left alone. Every cache
    /// that can be deleted is; the first that cannot (on Windows, one that a
    /// process has open or memory-mapped) is reported.
    pub fn clear(&self) -> Result<()> {
        let mut first_error = None;
        for directory in self.directories() {
            for path in self.cache_files_in(directory)? {
                match fs::remove_file(&path) {
                    Err(source) if source.kind() != ErrorKind::NotFound => {
                        first_error.get_or_insert(CfError::Io { path, source });
                    }
                    _ => {}
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    fn fingerprint(&self) -> Result<String> {
        SourceManifest::scan(&self.module_paths)?.fingerprint()
    }

    fn directories(&self) -> [&Path; 2] {
        [&self.in_corpus, &self.user_cache]
    }

    fn cache_file(&self, directory: &Path, fingerprint: &str) -> PathBuf {
        directory.join(format!("{}.{fingerprint}.{CACHE_EXTENSION}", self.stem))
    }

    /// Every cache of these module paths in `directory` (none when the
    /// directory does not exist).
    fn cache_files_in(&self, directory: &Path) -> Result<Vec<PathBuf>> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(source) if source.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(CfError::Io {
                    path: directory.to_path_buf(),
                    source,
                });
            }
        };
        Ok(entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| self.is_cache_name(&entry.file_name().to_string_lossy()))
            .map(|entry| entry.path())
            .collect())
    }

    /// Whether `name` is `<stem>.<fingerprint>.cfr`.
    fn is_cache_name(&self, name: &str) -> bool {
        name.strip_prefix(&self.stem)
            .and_then(|rest| rest.strip_prefix('.'))
            .and_then(|rest| rest.strip_suffix(CACHE_EXTENSION))
            .and_then(|rest| rest.strip_suffix('.'))
            .is_some_and(|fingerprint| {
                fingerprint.len() == FINGERPRINT_DIGITS
                    && fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
    }

    /// Whether `name` is a staged (still unpublished) cache of these module
    /// paths.
    fn is_staged_name(&self, name: &str) -> bool {
        name.strip_prefix('.')
            .and_then(|rest| rest.strip_prefix(&self.stem))
            .is_some_and(|rest| rest.starts_with('.') && rest.ends_with(STAGED_SUFFIX))
    }

    /// Best-effort removal, beside the freshly published `current` cache, of
    /// stale caches and of staged caches abandoned by compilers that died.
    /// A stale cache that cannot be deleted (on Windows, while a process has
    /// it mapped) is left for a later prune.
    fn prune_around(&self, current: &Path) {
        let Some(directory) = current.parent() else {
            return;
        };
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        let now = SystemTime::now();
        for entry in entries.filter_map(|entry| entry.ok()) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let stale = self.is_cache_name(&name) && path != current;
            let abandoned = self.is_staged_name(&name)
                && entry
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .ok()
                    .and_then(|modified| now.duration_since(modified).ok())
                    .is_some_and(|age| age > ABANDONED_STAGED_AGE);
            if stale || abandoned {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

/// Compiles the corpus (built once by `build`, then kept in `corpus` for any
/// later location) and publishes it at `cache_path`, unless another process
/// publishes a valid cache there first.
fn compile_into(
    cache_path: &Path,
    corpus: &mut Option<Corpus>,
    build: &impl Fn() -> Result<Corpus>,
) -> Result<()> {
    if let Some(directory) = cache_path.parent() {
        fs::create_dir_all(directory).map_err(|source| CfError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
    }
    // Staging before building finds an unwritable directory without the
    // cost of a build.
    let staged = StagedFile::new(cache_path)?;
    let corpus = match corpus {
        Some(corpus) => corpus,
        None => corpus.insert(build()?),
    };
    write_compiled_to(corpus, staged.file(), cache_path)?;
    match staged.publish_new()? {
        Publication::Published => Ok(()),
        // Another process compiled the same sources and published first.
        Publication::Occupied(discarded) if is_valid(cache_path) => {
            drop(discarded);
            Ok(())
        }
        // Something that is not a valid cache holds the name: replace it.
        Publication::Occupied(staged) => staged.publish_replacing().map_err(|error| match error {
            CfError::Io { path, source } => CfError::UnreplaceableCache { path, source },
            other => other,
        }),
    }
}

/// Whether `cache_path` holds a cache that opens in the current `.cfr`
/// format (a file in any other format, or a damaged one, is rebuilt).
fn is_valid(cache_path: &Path) -> bool {
    inspect_compiled(cache_path).is_ok()
}

fn is_location_unusable(error: &CfError) -> bool {
    match error {
        CfError::Io { source, .. } => matches!(
            source.kind(),
            ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem
        ),
        CfError::UnreplaceableCache { .. } => true,
        _ => false,
    }
}

fn canonical_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|path| fs::canonicalize(path).unwrap_or_else(|_| path.clone()))
        .collect()
}

/// Stable key identifying an ordered set of (canonical) module directories.
fn location_key(module_paths: &[PathBuf]) -> String {
    let mut joined = Vec::new();
    for path in module_paths {
        joined.extend_from_slice(path.to_string_lossy().as_bytes());
        joined.push(0);
    }
    format!("{:016x}", fnv1a_64(&joined))
}

/// 64-bit FNV-1a: a dependency-free hash that is stable across Rust versions
/// and platforms (unlike `std`'s `DefaultHasher`), as on-disk keys require.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET_BASIS, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::write_compiled;

    const MINI_CORPUS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/mini_corpus");

    /// A copy of the mini corpus in `into`, optionally without some features.
    fn copy_mini_corpus(into: &Path, without: &[&str]) -> PathBuf {
        fs::create_dir_all(into).unwrap();
        for entry in fs::read_dir(MINI_CORPUS).unwrap() {
            let path = entry.unwrap().path();
            let feature = path.file_stem().unwrap().to_string_lossy();
            if path.extension().is_some_and(|extension| extension == "tf")
                && !without.contains(&feature.as_ref())
            {
                fs::copy(&path, into.join(path.file_name().unwrap())).unwrap();
            }
        }
        into.to_path_buf()
    }

    fn build(module: &Path) -> impl Fn() -> Result<Corpus> {
        let module = module.to_path_buf();
        move || Ok(Corpus::load(&module)?.with_rank_arrays())
    }

    fn node_features(cache_path: &Path) -> Vec<String> {
        inspect_compiled(cache_path)
            .unwrap()
            .node_features
            .into_iter()
            .map(|feature| feature.name)
            .collect()
    }

    fn staged_files(directory: &Path) -> Vec<String> {
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(STAGED_SUFFIX))
            .collect()
    }

    /// A corpus copy, its cache (with the user cache under the same temporary
    /// directory), and the paths the current sources' cache gets in both
    /// locations.
    struct Fixture {
        _dir: tempfile::TempDir,
        corpus: PathBuf,
        cache: CorpusCache,
        in_corpus: PathBuf,
        in_user_cache: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let corpus = copy_mini_corpus(&dir.path().join("mini"), &[]);
            let cache =
                CorpusCache::with_user_cache(vec![corpus.clone()], &dir.path().join("user"));
            let fingerprint = cache.fingerprint().unwrap();
            let in_corpus = cache.cache_file(&cache.in_corpus, &fingerprint);
            let in_user_cache = cache.cache_file(&cache.user_cache, &fingerprint);
            Self {
                _dir: dir,
                corpus,
                cache,
                in_corpus,
                in_user_cache,
            }
        }

        /// Puts a directory holding a file at `path`: renaming a file over it
        /// or deleting it as a file fails on every platform, as replacing or
        /// deleting a memory-mapped file fails on Windows.
        fn block(path: &Path) {
            fs::create_dir_all(path).unwrap();
            fs::write(path.join("occupant"), b"").unwrap();
        }
    }

    #[test]
    fn fnv1a_matches_reference_vectors() {
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn another_engine_release_fingerprints_the_same_sources_differently() {
        let dir = tempfile::tempdir().unwrap();
        let corpus = copy_mini_corpus(&dir.path().join("mini"), &[]);
        let manifest = SourceManifest::scan(&[corpus]).unwrap();
        let mut older = manifest.clone();
        older.engine_version = "0.0.1".to_string();
        assert_ne!(
            manifest.fingerprint().unwrap(),
            older.fingerprint().unwrap()
        );
        assert_eq!(manifest.fingerprint().unwrap().len(), FINGERPRINT_DIGITS);
    }

    #[test]
    fn cache_names_belong_to_one_stem() {
        let single = CorpusCache::with_user_cache(vec![PathBuf::from("a")], Path::new("u"));
        let composed = CorpusCache::with_user_cache(
            vec![PathBuf::from("a"), PathBuf::from("b")],
            Path::new("u"),
        );
        let name = "corpus.0123456789abcdef.cfr";
        assert!(single.is_cache_name(name));
        assert!(!composed.is_cache_name(name));
        assert!(composed.is_cache_name(&format!("{}.0123456789abcdef.cfr", composed.stem)));
        assert!(!single.is_cache_name("corpus.cfr"));
        assert!(!single.is_cache_name("corpus.0123456789abcdef.cfr.partial"));
        assert!(single.is_staged_name(".corpus.0123456789abcdef.cfr-a1b2c3.partial"));
        assert!(!single.is_staged_name(&format!(".{}.0.cfr-x.partial", composed.stem)));
    }

    #[test]
    fn a_compiler_that_publishes_second_keeps_the_first_cache() {
        let fixture = Fixture::new();
        // A rival process publishes its cache while this one is building. Its
        // cache lacks `score`, so the test can tell whose cache survived.
        let rival_corpus = copy_mini_corpus(&fixture._dir.path().join("rival"), &["score"]);
        let rival = Corpus::load(&rival_corpus).unwrap().with_rank_arrays();
        let ours = build(&fixture.corpus);
        let path = fixture
            .cache
            .ensure_compiled(|| {
                write_compiled(&rival, &fixture.in_corpus)?;
                ours()
            })
            .unwrap();

        assert_eq!(path, fixture.in_corpus);
        assert!(!node_features(&path).contains(&"score".to_string()));
        assert_eq!(staged_files(&fixture.cache.in_corpus), Vec::<String>::new());
    }

    #[test]
    fn an_invalid_file_holding_the_name_is_replaced() {
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.cache.in_corpus).unwrap();
        fs::write(&fixture.in_corpus, b"CFRUST02 from an older release").unwrap();

        let path = fixture
            .cache
            .ensure_compiled(build(&fixture.corpus))
            .unwrap();
        assert_eq!(path, fixture.in_corpus);
        assert!(node_features(&path).contains(&"score".to_string()));
        assert_eq!(staged_files(&fixture.cache.in_corpus), Vec::<String>::new());
    }

    #[test]
    fn an_unreplaceable_name_falls_back_to_the_user_cache() {
        let fixture = Fixture::new();
        Fixture::block(&fixture.in_corpus);
        let built = std::cell::Cell::new(0);
        let corpus = build(&fixture.corpus);

        let path = fixture
            .cache
            .ensure_compiled(|| {
                built.set(built.get() + 1);
                corpus()
            })
            .unwrap();
        assert_eq!(path, fixture.in_user_cache);
        assert!(is_valid(&path));
        assert_eq!(
            built.get(),
            1,
            "the corpus is built once for both locations"
        );
        assert_eq!(staged_files(&fixture.cache.in_corpus), Vec::<String>::new());
    }

    #[test]
    fn unreplaceable_names_everywhere_fail_naming_the_preferred_file() {
        let fixture = Fixture::new();
        Fixture::block(&fixture.in_corpus);
        Fixture::block(&fixture.in_user_cache);

        let error = fixture
            .cache
            .ensure_compiled(build(&fixture.corpus))
            .unwrap_err();
        assert!(
            matches!(&error, CfError::UnreplaceableCache { path, .. } if path == &fixture.in_corpus),
            "{error}"
        );
        assert!(
            error
                .to_string()
                .contains("another process may have it open")
        );
        assert_eq!(staged_files(&fixture.cache.in_corpus), Vec::<String>::new());
        assert_eq!(
            staged_files(&fixture.cache.user_cache),
            Vec::<String>::new()
        );
    }

    #[test]
    fn changed_sources_get_a_new_cache_and_the_stale_one_is_pruned() {
        let fixture = Fixture::new();
        let first = fixture
            .cache
            .ensure_compiled(build(&fixture.corpus))
            .unwrap();
        fs::remove_file(fixture.corpus.join("score.tf")).unwrap();

        let second = fixture
            .cache
            .ensure_compiled(build(&fixture.corpus))
            .unwrap();
        assert_ne!(first, second);
        assert!(!first.exists());
        assert!(!node_features(&second).contains(&"score".to_string()));
    }

    #[test]
    fn pruning_removes_abandoned_staged_caches_only() {
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.cache.in_corpus).unwrap();
        let abandoned = fixture
            .cache
            .in_corpus
            .join(".corpus.0123456789abcdef.cfr-old.partial");
        let in_progress = fixture
            .cache
            .in_corpus
            .join(".corpus.0123456789abcdef.cfr-new.partial");
        for path in [&abandoned, &in_progress] {
            fs::write(path, b"").unwrap();
        }
        fs::File::options()
            .write(true)
            .open(&abandoned)
            .unwrap()
            .set_modified(SystemTime::now() - 2 * ABANDONED_STAGED_AGE)
            .unwrap();

        fixture
            .cache
            .ensure_compiled(build(&fixture.corpus))
            .unwrap();
        assert!(!abandoned.exists());
        assert!(in_progress.exists());
    }

    #[test]
    fn clear_deletes_every_cache_of_the_corpus() {
        let fixture = Fixture::new();
        let path = fixture
            .cache
            .ensure_compiled(build(&fixture.corpus))
            .unwrap();
        let unrelated = fixture.cache.in_corpus.join("notes.txt");
        fs::write(&unrelated, b"").unwrap();

        fixture.cache.clear().unwrap();
        assert!(!path.exists());
        assert!(unrelated.exists());
        fixture.cache.clear().unwrap();
    }

    #[test]
    fn clear_deletes_what_it_can_and_names_what_it_cannot() {
        let fixture = Fixture::new();
        let path = fixture
            .cache
            .ensure_compiled(build(&fixture.corpus))
            .unwrap();
        let undeletable = fixture.cache.in_corpus.join("corpus.0123456789abcdef.cfr");
        Fixture::block(&undeletable);

        let error = fixture.cache.clear().unwrap_err();
        assert!(
            matches!(&error, CfError::Io { path, .. } if path == &undeletable),
            "{error}"
        );
        assert!(!path.exists());
    }
}

mod user_dir;

pub use user_dir::user_cache_dir;
