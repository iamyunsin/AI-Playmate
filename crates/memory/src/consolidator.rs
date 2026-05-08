//! Background consolidation agent.
//!
//! Runs periodically (configurable interval) to:
//! 1. Fetch recent unprocessed messages from the episodic store.
//! 2. Ask the LLM to distill them into compact memory entries.
//! 3. Extract named entities and persist them in the entity graph.
//! 4. Store distilled entries in the semantic layer.
//!
//! This implements the "sleep-time compute" concept from Letta/MemGPT and the
//! hierarchical distillation approach used by mem0.

use chrono::{DateTime, Utc};
use tracing::{error, info, instrument};

use ai_playmate_core::{ChatSession, LLMConfig};

use crate::{
    entity::EntityGraph,
    episodic::EpisodicMemory,
    semantic::SemanticMemory,
    types::{EntityType, MemoryEntry},
    Result,
};

pub struct Consolidator {
    episodic: std::sync::Arc<EpisodicMemory>,
    semantic: std::sync::Arc<SemanticMemory>,
    entity_graph: std::sync::Arc<EntityGraph>,
    llm_config: LLMConfig,
    interval_secs: u64,
}

impl Consolidator {
    pub fn new(
        episodic: std::sync::Arc<EpisodicMemory>,
        semantic: std::sync::Arc<SemanticMemory>,
        entity_graph: std::sync::Arc<EntityGraph>,
        llm_config: LLMConfig,
        interval_secs: u64,
    ) -> Self {
        Self {
            episodic,
            semantic,
            entity_graph,
            llm_config,
            interval_secs,
        }
    }

    /// Spawn the consolidation loop as a Tokio background task.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut last_run = Utc::now() - chrono::Duration::hours(1);
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(self.interval_secs)).await;
                info!("Running memory consolidation");
                if let Err(e) = self.run_once(last_run).await {
                    error!("Consolidation error: {e}");
                } else {
                    last_run = Utc::now();
                }
            }
        })
    }

    #[instrument(skip(self, since))]
    async fn run_once(&self, since: DateTime<Utc>) -> Result<()> {
        let messages = self.episodic.messages_since(since).await?;
        if messages.is_empty() {
            return Ok(());
        }

        info!(count = messages.len(), "Consolidating messages");

        // Build a transcript to send to the LLM
        let transcript: String = messages
            .iter()
            .map(|m| format!("[{}]: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");

        let session = ChatSession::new(self.llm_config.clone());

        // ── Step 1: distil memories ─────────────────────────────────────
        let distil_prompt = format!(
            r#"You are a memory distillation assistant. Given the following conversation transcript,
extract 1-5 concise, standalone memory statements that capture important facts, preferences,
or events that should be remembered long-term. Output JSON array of strings only.

Transcript:
{transcript}

Output format: ["memory 1", "memory 2", ...]"#
        );

        let raw = session
            .complete(&distil_prompt, None, &[])
            .await
            .map_err(|e| crate::MemoryError::Other(e.to_string()))?;

        let memory_strings: Vec<String> = serde_json::from_str(raw.trim()).unwrap_or_default();
        let source_ids = messages.iter().map(|m| m.id).collect::<Vec<_>>();

        for content in memory_strings {
            let entry = MemoryEntry::new(content, source_ids.clone());
            self.semantic.store(entry).await?;
        }

        // ── Step 2: extract entities ─────────────────────────────────────
        let entity_prompt = format!(
            r#"Extract named entities (persons, places, topics, events, objects) from the transcript.
Output JSON array: [{{"name":"...","type":"person|place|topic|event|object","description":"..."}}]

Transcript:
{transcript}

Output format: [{{"name":"Alice","type":"person","description":"user's friend"}}]"#
        );

        let raw_entities = session
            .complete(&entity_prompt, None, &[])
            .await
            .map_err(|e| crate::MemoryError::Other(e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct ExtractedEntity {
            name: String,
            #[serde(rename = "type")]
            entity_type: String,
            description: Option<String>,
        }

        let extracted: Vec<ExtractedEntity> =
            serde_json::from_str(raw_entities.trim()).unwrap_or_default();

        let mut persisted_entities = Vec::new();
        for ext in extracted {
            let et = match ext.entity_type.as_str() {
                "person" => EntityType::Person,
                "place" => EntityType::Place,
                "topic" => EntityType::Topic,
                "event" => EntityType::Event,
                "object" => EntityType::Object,
                _ => EntityType::Other,
            };
            let entity = self
                .entity_graph
                .upsert_entity(&ext.name, et, ext.description)
                .await?;
            persisted_entities.push(entity);
        }

        info!(
            entities = persisted_entities.len(),
            "Consolidation complete"
        );
        Ok(())
    }
}
