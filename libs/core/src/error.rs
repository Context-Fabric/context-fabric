use std::path::PathBuf;

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

    #[error("missing required feature: {0}")]
    MissingFeature(String),

    #[error("invalid query: {0}")]
    InvalidQuery(String),

    #[error("invalid range specification: {0}")]
    InvalidSpec(String),

    #[error("regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
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
