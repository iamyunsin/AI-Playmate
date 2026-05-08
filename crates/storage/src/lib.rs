//! `ai-playmate-storage` — Storage adapters for AI Playmate.
//!
//! Provides two back-ends:
//! * **Vector store** (`QdrantStore`) — semantic similarity search via Qdrant.
//! * **Graph / document store** (`SurrealStore`) — entity graph, sessions, and
//!   key-value facts via SurrealDB (embedded RocksDB, no server required).

pub mod config;
pub mod graph;
pub mod vector;

pub use config::StorageConfig;
pub use graph::SurrealStore;
pub use vector::QdrantStore;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Qdrant error: {0}")]
    Qdrant(#[from] qdrant_client::QdrantError),

    #[error("SurrealDB error: {0}")]
    Surreal(#[from] surrealdb::Error),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Storage not initialised")]
    NotInitialised,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;
