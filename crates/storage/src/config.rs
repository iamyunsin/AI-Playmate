//! Storage configuration loaded from environment variables.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    // ── Qdrant ────────────────────────────────────────────────────────────
    /// Remote Qdrant URL, e.g. `http://localhost:6334`.
    /// When `None`, an in-process Qdrant Edge instance is used.
    pub qdrant_url: Option<String>,
    pub qdrant_api_key: Option<String>,
    /// Name of the Qdrant collection used for semantic memories.
    pub qdrant_collection: String,
    /// Dimension of the embedding vectors (must match your embedding model).
    pub vector_size: u64,

    // ── SurrealDB ─────────────────────────────────────────────────────────
    /// Remote SurrealDB URL, e.g. `ws://localhost:8000`.
    /// When `None`, an embedded RocksDB instance at `surreal_data_dir` is used.
    pub surreal_url: Option<String>,
    pub surreal_user: String,
    pub surreal_pass: String,
    /// Path used when running the embedded SurrealDB engine.
    pub surreal_data_dir: String,
    pub surreal_namespace: String,
    pub surreal_database: String,
}

impl StorageConfig {
    pub fn from_env() -> Self {
        Self {
            qdrant_url: std::env::var("QDRANT_URL").ok().filter(|s| !s.is_empty()),
            qdrant_api_key: std::env::var("QDRANT_API_KEY").ok().filter(|s| !s.is_empty()),
            qdrant_collection: std::env::var("QDRANT_COLLECTION")
                .unwrap_or_else(|_| "ai_playmate_memories".to_string()),
            vector_size: std::env::var("VECTOR_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(384), // BAAI/bge-small-en-v1.5

            surreal_url: std::env::var("SURREALDB_URL").ok().filter(|s| !s.is_empty()),
            surreal_user: std::env::var("SURREALDB_USER").unwrap_or_else(|_| "root".to_string()),
            surreal_pass: std::env::var("SURREALDB_PASS").unwrap_or_else(|_| "root".to_string()),
            surreal_data_dir: std::env::var("SURREALDB_DATA_DIR")
                .unwrap_or_else(|_| "./data/surrealdb".to_string()),
            surreal_namespace: std::env::var("SURREALDB_NAMESPACE")
                .unwrap_or_else(|_| "ai_playmate".to_string()),
            surreal_database: std::env::var("SURREALDB_DATABASE")
                .unwrap_or_else(|_| "main".to_string()),
        }
    }
}
