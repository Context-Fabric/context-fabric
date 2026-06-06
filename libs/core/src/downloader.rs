use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use crate::error::{CfError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusRegistryEntry {
    pub repo_id: String,
    pub description: String,
    pub language: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadRequest {
    pub repo_id: String,
    pub revision: Option<String>,
    pub force: bool,
    pub compiled_only: bool,
    pub allow_patterns: Option<Vec<String>>,
}

pub fn corpus_registry() -> BTreeMap<String, CorpusRegistryEntry> {
    BTreeMap::new()
}

#[allow(non_snake_case)]
pub fn CORPUS_REGISTRY() -> BTreeMap<String, CorpusRegistryEntry> {
    corpus_registry()
}

pub fn list_corpora() -> BTreeMap<String, CorpusRegistryEntry> {
    corpus_registry()
}

pub fn resolve_corpus_id(corpus_id: &str) -> Result<String> {
    if corpus_id.contains('/') {
        return Ok(corpus_id.to_string());
    }

    let registry = corpus_registry();
    if let Some(entry) = registry.get(corpus_id) {
        return Ok(entry.repo_id.clone());
    }

    Err(CfError::InvalidSpec(format!(
        "Unknown corpus: {corpus_id}. Use list_corpora() to see available corpora, or provide a full HF repo ID (e.g., 'username/cfabric-corpus')."
    )))
}

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

#[allow(non_snake_case)]
pub fn getCacheDir() -> PathBuf {
    get_cache_dir()
}

pub fn clear_cache(_corpus_id: Option<&str>) -> Result<()> {
    Err(CfError::InvalidSpec(
        "Cache clearing not yet implemented. Use huggingface_hub's cache management for now."
            .to_string(),
    ))
}

pub fn build_download_request(
    corpus_id: &str,
    revision: Option<&str>,
    force: bool,
    compiled_only: bool,
) -> Result<DownloadRequest> {
    Ok(DownloadRequest {
        repo_id: resolve_corpus_id(corpus_id)?,
        revision: revision.map(str::to_string),
        force,
        compiled_only,
        allow_patterns: compiled_only.then(|| {
            vec![
                ".cfm/**".to_string(),
                "corpus_info.json".to_string(),
                "README.md".to_string(),
            ]
        }),
    })
}

pub fn download(
    corpus_id: &str,
    revision: Option<&str>,
    force: bool,
    compiled_only: bool,
) -> Result<PathBuf> {
    let request = build_download_request(corpus_id, revision, force, compiled_only)?;
    Err(CfError::InvalidSpec(format!(
        "Rust downloader transport is not implemented yet for repo {}",
        request.repo_id
    )))
}

fn dirs_home() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
