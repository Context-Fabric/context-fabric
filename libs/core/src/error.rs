//! [`CfError`], the error type of every fallible operation.

use std::path::PathBuf;

use crate::search::TemplateErrors;

#[derive(Debug, thiserror::Error)]
pub enum CfError {
    #[error("I/O error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{path}:{line}: {message}")]
    Parse {
        path: PathBuf,
        line: usize,
        message: String,
    },

    #[error(
        "{path} is a compiled cache in .cfr format {found}; this release reads format \
         {expected}. Delete it to have Fabric.load rebuild it, or recompile it with \
         Fabric.compile."
    )]
    IncompatibleCache {
        path: PathBuf,
        found: String,
        expected: &'static str,
    },

    /// The name a compiled cache is published under is held by a file that is
    /// not a valid cache and cannot be replaced (on Windows, a file another
    /// process has open or memory-mapped cannot be replaced).
    #[error(
        "{path} is not a valid compiled cache and cannot be replaced ({source}); another \
         process may have it open. Delete it once it is closed, then load again."
    )]
    UnreplaceableCache {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("missing required feature: {0}")]
    MissingFeature(String),

    /// A bad search template, with Text-Fabric's messages.
    #[error("{0}")]
    Template(TemplateErrors),

    /// A running search was stopped by its [`Interrupt`](crate::search::Interrupt).
    #[error("search interrupted")]
    Interrupted,

    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl CfError {
    /// A bad search template, found while planning it.
    pub(crate) fn template(message: impl Into<String>) -> Self {
        Self::Template(TemplateErrors::plan(message))
    }
}

impl From<std::io::Error> for CfError {
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            path: PathBuf::new(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, CfError>;
