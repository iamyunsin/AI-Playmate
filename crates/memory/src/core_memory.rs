//! Core memory — key facts about the user that are always in the LLM context window.
//!
//! Limited in size (default 2 000 tokens) and stored persistently in SurrealDB
//! under the key `core_memory:current`.  The LLM can update it explicitly.

use serde::{Deserialize, Serialize};
use tracing::debug;

use ai_playmate_storage::SurrealStore;

use crate::Result;

const CORE_MEMORY_ID: &str = "current";
const CORE_MEMORY_TABLE: &str = "core_memory";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreMemoryRecord {
    pub user_profile: String,
    pub ai_persona: String,
    pub important_facts: Vec<String>,
}

impl Default for CoreMemoryRecord {
    fn default() -> Self {
        Self {
            user_profile: "No profile information yet.".to_string(),
            ai_persona: "I am a friendly and empathetic AI companion with a good memory."
                .to_string(),
            important_facts: Vec::new(),
        }
    }
}

pub struct CoreMemory {
    store: std::sync::Arc<SurrealStore>,
    max_tokens: usize,
}

impl CoreMemory {
    pub fn new(store: std::sync::Arc<SurrealStore>, max_tokens: usize) -> Self {
        Self { store, max_tokens }
    }

    pub async fn get(&self) -> Result<CoreMemoryRecord> {
        let record: Option<CoreMemoryRecord> = self
            .store
            .get(CORE_MEMORY_TABLE, CORE_MEMORY_ID)
            .await?;
        Ok(record.unwrap_or_default())
    }

    pub async fn update(&self, record: CoreMemoryRecord) -> Result<()> {
        debug!("Updating core memory");
        self.store
            .upsert(CORE_MEMORY_TABLE, CORE_MEMORY_ID, record)
            .await?;
        Ok(())
    }

    /// Append a new important fact, trimming old ones if max_tokens is exceeded.
    pub async fn add_fact(&self, fact: &str) -> Result<()> {
        let mut record = self.get().await?;
        record.important_facts.push(fact.to_string());

        // Simple token budget guard: estimate 4 chars per token
        let total_chars: usize = record.important_facts.iter().map(|f| f.len()).sum();
        while total_chars > self.max_tokens * 4 && !record.important_facts.is_empty() {
            record.important_facts.remove(0); // drop oldest
        }

        self.update(record).await
    }

    /// Render as a string for prompt injection.
    pub async fn to_prompt_string(&self) -> Result<String> {
        let record = self.get().await?;
        let facts = if record.important_facts.is_empty() {
            "(none yet)".to_string()
        } else {
            record
                .important_facts
                .iter()
                .enumerate()
                .map(|(i, f)| format!("{}. {}", i + 1, f))
                .collect::<Vec<_>>()
                .join("\n")
        };

        Ok(format!(
            "## User Profile\n{}\n\n## AI Persona\n{}\n\n## Important Facts\n{}",
            record.user_profile, record.ai_persona, facts
        ))
    }
}
