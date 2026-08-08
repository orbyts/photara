use std::{io, path::PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, PhotaraError>;

#[derive(Debug, Error)]
pub enum PhotaraError {
    #[error("Photara configuration error: {0}")]
    Configuration(String),

    #[error("could not {action} {path}")]
    Filesystem {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("could not parse TOML configuration {path}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("could not process YAML registry {path}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("could not serialize project manifest")]
    Json(#[from] serde_json::Error),

    #[error("could not parse selection CSV")]
    Csv(#[from] csv::Error),

    #[error(transparent)]
    Storexa(#[from] storexa::StorexaError),

    #[error("Photara database query failed")]
    Database(#[from] sqlx::Error),
}

impl PhotaraError {
    pub(crate) fn filesystem(
        action: &'static str,
        path: impl Into<PathBuf>,
        source: io::Error,
    ) -> Self {
        Self::Filesystem {
            action,
            path: path.into(),
            source,
        }
    }
}
