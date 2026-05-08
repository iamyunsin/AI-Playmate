//! Application state shared across Tauri commands.

use std::sync::Arc;

use ai_playmate_core::{ChatSession, LLMConfig};
use ai_playmate_memory::{MemoryConfig, MemorySystem};

/// Global application state, stored inside a `tauri::State`.
pub struct AppState {
    pub memory: Arc<MemorySystem>,
    pub chat_session: Arc<ChatSession>,
}

impl AppState {
    pub async fn init() -> anyhow::Result<Self> {
        // Load config from environment (reads .env if present via `dotenvy`)
        let memory_config = MemoryConfig::from_env()
            .map_err(|e| anyhow::anyhow!("Failed to build memory config: {e}"))?;
        let llm_config = LLMConfig::from_env()
            .map_err(|e| anyhow::anyhow!("Failed to build LLM config: {e}"))?;

        let memory = Arc::new(
            MemorySystem::new(memory_config)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to init memory system: {e}"))?,
        );

        let chat_session = Arc::new(ChatSession::new(llm_config));

        Ok(Self {
            memory,
            chat_session,
        })
    }
}
