//! [`MemorySystem`] — top-level façade that wires all memory layers together.

use std::sync::Arc;

use ai_playmate_core::{EmbeddingProvider, LLMConfig, LocalEmbedder, OllamaEmbedder};
use ai_playmate_storage::{QdrantStore, StorageConfig, SurrealStore};

use crate::{
    consolidator::Consolidator,
    core_memory::CoreMemory,
    entity::EntityGraph,
    episodic::EpisodicMemory,
    retriever::AssociativeRetriever,
    semantic::SemanticMemory,
    types::{Message, RetrievedContext},
    Result,
};

/// Configuration for the memory system.
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub storage: StorageConfig,
    pub llm: LLMConfig,
    /// Max tokens for core (always-in-context) memory.
    pub core_max_tokens: usize,
    /// Number of recent messages kept verbatim in context.
    pub episodic_window: usize,
    /// Number of semantic memories injected per turn.
    pub semantic_top_k: usize,
    /// Seconds between background consolidation runs.
    pub consolidation_interval_secs: u64,
}

impl MemoryConfig {
    pub fn from_env() -> crate::Result<Self> {
        let storage = StorageConfig::from_env();
        let llm = LLMConfig::from_env()?;

        Ok(Self {
            storage,
            llm,
            core_max_tokens: std::env::var("MEMORY_CORE_MAX_TOKENS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2000),
            episodic_window: std::env::var("MEMORY_EPISODIC_WINDOW")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(50),
            semantic_top_k: std::env::var("MEMORY_SEMANTIC_TOP_K")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            consolidation_interval_secs: std::env::var("MEMORY_CONSOLIDATION_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
        })
    }
}

/// The central orchestrator of the memory system.
///
/// Holds references to all sub-systems and exposes the two primary operations:
/// * [`store_message`] — persist a message and trigger async indexing.
/// * [`retrieve_context`] — fetch relevant context for LLM injection.
pub struct MemorySystem {
    pub core: Arc<CoreMemory>,
    pub episodic: Arc<EpisodicMemory>,
    pub semantic: Arc<SemanticMemory>,
    pub entity_graph: Arc<EntityGraph>,
    pub retriever: Arc<AssociativeRetriever>,
    _consolidation_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MemorySystem {
    /// Initialise all storage backends and start the background consolidator.
    pub async fn new(config: MemoryConfig) -> Result<Self> {
        // ── Storage layer ────────────────────────────────────────────────
        let surreal = Arc::new(SurrealStore::new(&config.storage).await?);

        // Qdrant is optional — if unavailable, semantic memory is disabled.
        let qdrant_result = QdrantStore::new(&config.storage).await;
        let qdrant_opt: Option<Arc<QdrantStore>> = match qdrant_result {
            Ok(q) => Some(Arc::new(q)),
            Err(e) => {
                tracing::warn!(
                    "Qdrant unavailable — semantic memory disabled. \
                     Start Qdrant locally or set QDRANT_URL. Error: {e}"
                );
                None
            }
        };

        // Embedder is only needed when Qdrant is available.
        // Prefer OllamaEmbedder (no download needed) when OLLAMA_BASE_URL is set.
        // Fall back to LocalEmbedder (fastembed) if Ollama embedding is unavailable.
        let embedder_opt: Option<Arc<dyn EmbeddingProvider>> = if qdrant_opt.is_some() {
            let ollama_url = std::env::var("OLLAMA_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string());
            let ollama_embed_model = std::env::var("OLLAMA_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "qwen3-embedding:8b".to_string());
            let ollama_embed_dim = std::env::var("OLLAMA_EMBEDDING_DIM")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(4096);
            let embedder = OllamaEmbedder::new(ollama_url, ollama_embed_model, ollama_embed_dim);
            // Quick smoke-test to verify connectivity.
            match embedder.embed("test").await {
                Ok(_) => {
                    tracing::info!("OllamaEmbedder ready");
                    Some(Arc::new(embedder) as Arc<dyn EmbeddingProvider>)
                }
                Err(e) => {
                    tracing::warn!("OllamaEmbedder unavailable ({e}), trying LocalEmbedder");
                    match LocalEmbedder::new() {
                        Ok(e) => Some(Arc::new(e) as Arc<dyn EmbeddingProvider>),
                        Err(e) => {
                            tracing::warn!("LocalEmbedder also failed — semantic memory disabled: {e}");
                            None
                        }
                    }
                }
            }
        } else {
            None
        };

        // ── Memory layers ────────────────────────────────────────────────
        let core = Arc::new(CoreMemory::new(Arc::clone(&surreal), config.core_max_tokens));
        let episodic = Arc::new(EpisodicMemory::new(Arc::clone(&surreal), config.episodic_window));
        let semantic = Arc::new(SemanticMemory::new(
            qdrant_opt,
            Arc::clone(&surreal),
            embedder_opt,
            config.semantic_top_k,
        ));
        let entity_graph = Arc::new(EntityGraph::new(Arc::clone(&surreal)));

        // ── Retriever ────────────────────────────────────────────────────
        let retriever = Arc::new(AssociativeRetriever::new(
            Arc::clone(&core),
            Arc::clone(&episodic),
            Arc::clone(&semantic),
            Arc::clone(&entity_graph),
            4096,
        ));

        // ── Background consolidator ──────────────────────────────────────
        let consolidation_handle = {
            let consolidator = Consolidator::new(
                Arc::clone(&episodic),
                Arc::clone(&semantic),
                Arc::clone(&entity_graph),
                config.llm,
                config.consolidation_interval_secs,
            );
            Some(consolidator.spawn())
        };

        Ok(Self {
            core,
            episodic,
            semantic,
            entity_graph,
            retriever,
            _consolidation_handle: consolidation_handle,
        })
    }

    /// Persist a new message and return immediately (indexing is async).
    pub async fn store_message(&self, message: &Message) -> Result<()> {
        self.episodic.store_message(message).await
    }

    /// Retrieve context relevant to the current `query` in `session_id`.
    pub async fn retrieve_context(
        &self,
        query: &str,
        session_id: uuid::Uuid,
    ) -> Result<RetrievedContext> {
        self.retriever.retrieve(query, session_id).await
    }
}
