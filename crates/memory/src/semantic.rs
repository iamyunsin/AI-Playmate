//! Semantic memory — compressed memories stored as vectors in Qdrant.
//!
//! Distilled [`MemoryEntry`] objects are embedded and stored here.  At
//! retrieval time a cosine-similarity search returns the most relevant
//! memories given the current query.

use tracing::{debug, instrument};
use uuid::Uuid;

use ai_playmate_core::EmbeddingProvider;
use ai_playmate_storage::{QdrantStore, SurrealStore};

use crate::{types::MemoryEntry, MemoryError, Result};

const MEMORY_TABLE: &str = "memory_entry";

pub struct SemanticMemory {
    vector_store: Option<std::sync::Arc<QdrantStore>>,
    graph_store: std::sync::Arc<SurrealStore>,
    embedder: Option<std::sync::Arc<dyn EmbeddingProvider>>,
    top_k: usize,
}

impl SemanticMemory {
    pub fn new(
        vector_store: Option<std::sync::Arc<QdrantStore>>,
        graph_store: std::sync::Arc<SurrealStore>,
        embedder: Option<std::sync::Arc<dyn EmbeddingProvider>>,
        top_k: usize,
    ) -> Self {
        Self {
            vector_store,
            graph_store,
            embedder,
            top_k,
        }
    }

    /// Embed and store a [`MemoryEntry`].
    #[instrument(skip(self, entry))]
    pub async fn store(&self, mut entry: MemoryEntry) -> Result<MemoryEntry> {
        let (Some(vs), Some(emb)) = (&self.vector_store, &self.embedder) else {
            // Qdrant unavailable — persist metadata only
            self.graph_store
                .upsert(MEMORY_TABLE, &entry.id.to_string(), entry.clone())
                .await?;
            return Ok(entry);
        };

        let vector = emb.embed(&entry.content).await?;
        entry.embedding = Some(vector.clone());

        self.graph_store
            .upsert(MEMORY_TABLE, &entry.id.to_string(), entry.clone())
            .await?;

        let payload = serde_json::to_value(&entry).map_err(MemoryError::Json)?;
        vs.upsert(entry.id, vector, payload).await?;

        debug!(id = %entry.id, "Stored semantic memory");
        Ok(entry)
    }

    /// Find the `top_k` memories semantically closest to `query`.
    #[instrument(skip(self, query))]
    pub async fn search(&self, query: &str, top_k: Option<usize>) -> Result<Vec<MemoryEntry>> {
        let (Some(vs), Some(emb)) = (&self.vector_store, &self.embedder) else {
            return Ok(vec![]);
        };

        let k = top_k.unwrap_or(self.top_k) as u64;
        let query_vec = emb.embed(query).await?;
        let hits = vs.search(query_vec, k, Some(0.3)).await?;

        let mut results = Vec::with_capacity(hits.len());
        for (_id, _score, payload) in hits {
            let entry: MemoryEntry = serde_json::from_value(payload).map_err(MemoryError::Json)?;
            results.push(entry);
        }

        Ok(results)
    }

    /// Update the `last_accessed_at` and `access_count` of a stored memory.
    pub async fn record_access(&self, id: Uuid) -> Result<()> {
        if let Some(mut entry) = self.graph_store.get::<MemoryEntry>(MEMORY_TABLE, &id.to_string()).await? {
            entry.record_access();
            self.graph_store
                .upsert(MEMORY_TABLE, &id.to_string(), entry.clone())
                .await?;
        }
        Ok(())
    }
}
