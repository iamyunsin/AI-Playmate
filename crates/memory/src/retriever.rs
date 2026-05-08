//! Associative retriever — fuses episodic, semantic, and entity-graph results.
//!
//! ## Retrieval pipeline
//!
//! 1. Embed the user query.
//! 2. Semantic search in Qdrant for compressed memories.
//! 3. Entity name scan in SurrealDB; graph-expand to related entities.
//! 4. Load the most recent N turns from EpisodicMemory.
//! 5. Merge & de-duplicate, apply a recency / importance score, and trim to
//!    fit within the token budget.

use tracing::{debug, instrument};

use crate::{
    core_memory::CoreMemory,
    entity::EntityGraph,
    episodic::EpisodicMemory,
    semantic::SemanticMemory,
    types::RetrievedContext,
    Result,
};
use uuid::Uuid;

pub struct AssociativeRetriever {
    core: std::sync::Arc<CoreMemory>,
    episodic: std::sync::Arc<EpisodicMemory>,
    semantic: std::sync::Arc<SemanticMemory>,
    entity_graph: std::sync::Arc<EntityGraph>,
    max_context_tokens: usize,
}

impl AssociativeRetriever {
    pub fn new(
        core: std::sync::Arc<CoreMemory>,
        episodic: std::sync::Arc<EpisodicMemory>,
        semantic: std::sync::Arc<SemanticMemory>,
        entity_graph: std::sync::Arc<EntityGraph>,
        max_context_tokens: usize,
    ) -> Self {
        Self {
            core,
            episodic,
            semantic,
            entity_graph,
            max_context_tokens,
        }
    }

    /// Build a [`RetrievedContext`] relevant to `query` for the given `session_id`.
    #[instrument(skip(self, query))]
    pub async fn retrieve(
        &self,
        query: &str,
        session_id: Uuid,
    ) -> Result<RetrievedContext> {
        let mut ctx = RetrievedContext::default();

        // 1. Recent episodic turns (always included first)
        ctx.recency_context = self.episodic.recent_messages(session_id).await?;

        // 2. Semantic memory search
        let semantic_hits = self.semantic.search(query, None).await?;
        // Record access so frequently retrieved memories gain importance
        for hit in &semantic_hits {
            let _ = self.semantic.record_access(hit.id).await;
        }
        ctx.memories = semantic_hits;

        // 3. Entity graph expansion: find named entities in the query
        let entity_hits = self.entity_graph.search_by_name(query, 3).await?;
        let mut related: Vec<_> = entity_hits.clone();
        for entity in &entity_hits {
            let expanded = self.entity_graph.related_entities(entity.id, 1).await?;
            related.extend(expanded);
        }
        // De-duplicate by id
        related.dedup_by_key(|e| e.id);
        ctx.relevant_entities = related;

        // 4. Rough token estimate (4 chars ≈ 1 token)
        let chars: usize = ctx
            .recency_context
            .iter()
            .map(|m| m.content.len())
            .sum::<usize>()
            + ctx.memories.iter().map(|m| m.content.len()).sum::<usize>()
            + ctx
                .relevant_entities
                .iter()
                .map(|e| e.name.len() + e.description.as_deref().unwrap_or("").len())
                .sum::<usize>();
        ctx.estimated_tokens = chars / 4;

        debug!(
            memories = ctx.memories.len(),
            entities = ctx.relevant_entities.len(),
            recency = ctx.recency_context.len(),
            estimated_tokens = ctx.estimated_tokens,
            "Assembled retrieval context"
        );

        Ok(ctx)
    }
}
