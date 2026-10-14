//! Location and freshness of the compiled `.cfr` cache that `Fabric.load`
//! builds from a corpus's `.tf` sources.
//!
//! A cache is identified by the ordered list of module directories it was
//! compiled from and is considered fresh only while a [`SourceManifest`] of
//! those directories (every `.tf` file's name, size, and modification time)
//! matches the manifest written next to the cache. That catches edited,
//! added, deleted, and replaced-with-older-mtime feature files, and keeps a
//! cache compiled from one location set from standing in for another.
//!
//! Caches live inside the last module directory (`<module>/.cfr/<version>/`)
//! so they travel with the corpus. When that directory is not writable, the
//! cache is placed under the user cache directory instead
//! (`<cache dir>/cfr/<location key>/<version>/`).

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::atomic_file::write_atomically;
use crate::compiled::inspect_compiled;
use crate::config::{CFR_VERSION, VERSION};
use crate::downloader::get_cache_dir;
use crate::error::{CfError, Result};

const CACHE_DIR_NAME: &str = ".cfr";
const CACHE_FILE_STEM: &str = "corpus";
const CACHE_EXTENSION: &str = "cfr";
const MANIFEST_SUFFIX: &str = ".sources.json";

/// One `.tf` source file as seen when a cache was compiled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SourceFile {
    /// Index of the module directory in [`SourceManifest::module_paths`].
    module: usize,
    name: String,
    len: u64,
    modified_ns: u128,
}

/// Snapshot of the `.tf` sources a compiled cache is built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest {
    cfr_version: String,
    /// Engine release that wrote the cache: a new release recompiles once, so
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

    /// Whether the manifest stored beside `cache_path` equals this one.
    fn matches_cache(&self, cache_path: &Path) -> bool {
        fs::read(manifest_path(cache_path))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<SourceManifest>(&bytes).ok())
            .is_some_and(|stored| &stored == self)
    }

    /// Atomically writes this manifest beside `cache_path`.
    fn write_for_cache(&self, cache_path: &Path) -> Result<()> {
        write_atomically(&manifest_path(cache_path), |file| {
            serde_json::to_writer(file, self)?;
            Ok(())
        })
    }
}

/// Where a corpus's compiled cache may live, in order of preference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheLocations {
    /// Inside the last module directory, travelling with the corpus.
    pub in_corpus: PathBuf,
    /// Under the user cache directory, for read-only corpus directories.
    pub user_cache: PathBuf,
}

impl CacheLocations {
    pub fn for_module_paths(module_paths: &[PathBuf]) -> Self {
        // The key is derived from canonical paths so `./corpus` and its
        // absolute spelling share a cache; the in-corpus path keeps the
        // caller's spelling.
        let key = location_key(&canonical_paths(module_paths));
        let last_module = module_paths
            .last()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("."));
        // One location keeps the conventional name; a composition of several
        // gets its own file so it never overwrites the single-location cache.
        let file_name = if module_paths.len() > 1 {
            format!("{CACHE_FILE_STEM}-{key}.{CACHE_EXTENSION}")
        } else {
            format!("{CACHE_FILE_STEM}.{CACHE_EXTENSION}")
        };
        Self {
            in_corpus: last_module
                .join(CACHE_DIR_NAME)
                .join(CFR_VERSION)
                .join(&file_name),
            user_cache: get_cache_dir()
                .join("cfr")
                .join(key)
                .join(CFR_VERSION)
                .join(&file_name),
        }
    }

    fn candidates(&self) -> [&Path; 2] {
        [&self.in_corpus, &self.user_cache]
    }
}

/// Returns the path of a fresh compiled cache for `module_paths`, compiling
/// one with `compile` when none of the [`CacheLocations`] holds a fresh cache.
///
/// `compile` receives the destination path and must write a complete cache
/// there. A location whose directory cannot be written (permission denied or
/// read-only file system) is skipped in favour of the next.
pub fn ensure_compiled(
    module_paths: &[PathBuf],
    compile: impl Fn(&Path) -> Result<()>,
) -> Result<PathBuf> {
    let manifest = SourceManifest::scan(module_paths)?;
    let locations = CacheLocations::for_module_paths(module_paths);
    for candidate in locations.candidates() {
        if is_fresh(candidate, &manifest) {
            return Ok(candidate.to_path_buf());
        }
    }
    let mut first_error = None;
    for candidate in locations.candidates() {
        match compile_into(candidate, &manifest, &compile) {
            Ok(()) => return Ok(candidate.to_path_buf()),
            Err(error) if is_unwritable(&error) => {
                first_error.get_or_insert(error);
            }
            Err(error) => return Err(error),
        }
    }
    Err(first_error.expect("at least one cache location was attempted"))
}

fn compile_into(
    cache_path: &Path,
    manifest: &SourceManifest,
    compile: &impl Fn(&Path) -> Result<()>,
) -> Result<()> {
    if let Some(directory) = cache_path.parent() {
        fs::create_dir_all(directory).map_err(|source| CfError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
    }
    compile(cache_path).map_err(|error| match error {
        CfError::Io { source, .. } => CfError::Io {
            path: cache_path.to_path_buf(),
            source,
        },
        other => other,
    })?;
    manifest.write_for_cache(cache_path)
}

fn is_fresh(cache_path: &Path, manifest: &SourceManifest) -> bool {
    manifest.matches_cache(cache_path) && inspect_compiled(cache_path).is_ok()
}

fn is_unwritable(error: &CfError) -> bool {
    matches!(
        error,
        CfError::Io { source, .. }
            if matches!(
                source.kind(),
                ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem
            )
    )
}

fn manifest_path(cache_path: &Path) -> PathBuf {
    let mut name = cache_path.as_os_str().to_owned();
    name.push(MANIFEST_SUFFIX);
    PathBuf::from(name)
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

    #[test]
    fn fnv1a_matches_reference_vectors() {
        assert_eq!(fnv1a_64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a_64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn a_cache_written_by_another_engine_release_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        let module = dir.path().join("corpus");
        fs::create_dir_all(&module).unwrap();
        fs::write(module.join("otype.tf"), "@node\n\n1\tword\n").unwrap();
        let cache = dir.path().join("corpus.cfr");
        let manifest = SourceManifest::scan(&[module]).unwrap();
        manifest.write_for_cache(&cache).unwrap();
        assert!(manifest.matches_cache(&cache));

        let mut older = manifest.clone();
        older.engine_version = "0.0.1".to_string();
        older.write_for_cache(&cache).unwrap();
        assert!(!manifest.matches_cache(&cache));
    }

    #[test]
    fn manifest_sidecar_sits_beside_the_cache() {
        assert_eq!(
            manifest_path(Path::new("/c/.cfr/3/corpus.cfr")),
            PathBuf::from("/c/.cfr/3/corpus.cfr.sources.json")
        );
    }
}
