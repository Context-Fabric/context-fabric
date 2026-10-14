use std::env;
use std::path::PathBuf;

pub fn get_cache_dir() -> PathBuf {
    if let Ok(path) = env::var("CFABRIC_CACHE") {
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }

    if cfg!(target_os = "macos") {
        return dirs_home().join("Library").join("Caches").join("cfabric");
    }

    if cfg!(target_os = "windows") {
        if let Ok(path) = env::var("LOCALAPPDATA") {
            if !path.is_empty() {
                return PathBuf::from(path)
                    .join("cfabric")
                    .join("cfabric")
                    .join("Cache");
            }
        }
    }

    if let Ok(path) = env::var("XDG_CACHE_HOME") {
        if !path.is_empty() {
            return PathBuf::from(path).join("cfabric");
        }
    }

    dirs_home().join(".cache").join("cfabric")
}

fn dirs_home() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
