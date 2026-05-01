use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OmelaError {
    #[error("I/O error on {path}: {source}")]
    Io { path: PathBuf, source: io::Error },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Invalid tree path: {0:?}")]
    InvalidPath(Vec<usize>),

    #[error("{0}")]
    Other(String),
}

impl OmelaError {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

pub type Result<T> = std::result::Result<T, OmelaError>;
