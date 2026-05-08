//! `ai-playmate-memory` — Three-layer associative memory system.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │  MemorySystem (top-level orchestrator)                  │
//! │                                                         │
//! │  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
//! │  │ CoreMemory  │  │EpisodicMemory│  │SemanticMemory │  │
//! │  │(always in   │  │(recent turns,│  │(compressed    │  │
//! │  │ context)    │  │ SurrealDB)   │  │ vectors,      │  │
//! │  └─────────────┘  └──────────────┘  │ Qdrant)       │  │
//! │                                     └───────────────┘  │
//! │  ┌─────────────────────────────────────────────────┐   │
//! │  │ EntityGraph (named entity relations, SurrealDB) │   │
//! │  └─────────────────────────────────────────────────┘   │
//! │                                                         │
//! │  AssociativeRetriever  ←─ fuses all three layers        │
//! │  Consolidator          ←─ background compression agent  │
//! └─────────────────────────────────────────────────────────┘
//! ```

pub mod consolidator;
pub mod core_memory;
pub mod entity;
pub mod episodic;
pub mod retriever;
pub mod semantic;
pub mod system;
pub mod types;

pub use system::{MemoryConfig, MemorySystem};
pub use types::{
    Entity, EntityRelation, EntityType, MemoryEntry, Message, MessageRole, RetrievedContext,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Storage error: {0}")]
    Storage(#[from] ai_playmate_storage::StorageError),

    #[error("Core AI error: {0}")]
    Core(#[from] ai_playmate_core::CoreError),

    #[error("Serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, MemoryError>;
