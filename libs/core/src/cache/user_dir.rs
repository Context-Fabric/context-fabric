//! The per-user cache directory: compiled caches of read-only corpora live
//! here, and Python exposes it as `cfabric.get_cache_dir()`.

use std::env;
use std::path::PathBuf;

/// `CFABRIC_CACHE` when set, else the platform's user cache directory for
/// `cfabric` (the location `platformdirs.user_cache_dir("cfabric", "cfabric")`
/// gives): `~/Library/Caches/cfabric` on macOS,
/// `%LOCALAPPDATA%\cfabric\cfabric\Cache` on Windows, and
/// `$XDG_CACHE_HOME/cfabric` or `~/.cache/cfabric` elsewhere.
pub fn user_cache_dir() -> PathBuf {
    if let Some(path) = non_empty_var("CFABRIC_CACHE") {
        return PathBuf::from(path);
    }
    if cfg!(target_os = "macos") {
        return home_dir().join("Library").join("Caches").join("cfabric");
    }
    if cfg!(target_os = "windows")
        && let Some(local_app_data) = non_empty_var("LOCALAPPDATA")
    {
        return PathBuf::from(local_app_data)
            .join("cfabric")
            .join("cfabric")
            .join("Cache");
    }
    if let Some(cache_home) = non_empty_var("XDG_CACHE_HOME") {
        return PathBuf::from(cache_home).join("cfabric");
    }
    home_dir().join(".cache").join("cfabric")
}

fn non_empty_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn home_dir() -> PathBuf {
    non_empty_var("HOME")
        .or_else(|| non_empty_var("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}
