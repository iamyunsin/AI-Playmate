//! Error types for `ai-playmate-core`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("LLM provider error: {0}")]
    Provider(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for CoreError {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e.to_string())
    }
}
