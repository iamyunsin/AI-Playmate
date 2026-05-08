//! Episodic memory — raw conversation turns stored in SurrealDB.
//!
//! Acts as the "recall memory" layer: recent messages are kept verbatim and
//! injected into the context window up to a configurable sliding window.

use chrono::Utc;
use tracing::instrument;
use uuid::Uuid;

use ai_playmate_storage::SurrealStore;

use crate::{
    types::Message,
    Result,
};

const MESSAGE_TABLE: &str = "message";
const SESSION_TABLE: &str = "session";

pub struct EpisodicMemory {
    store: std::sync::Arc<SurrealStore>,
    /// Number of recent messages to keep in the sliding context window.
    window_size: usize,
}

impl EpisodicMemory {
    pub fn new(store: std::sync::Arc<SurrealStore>, window_size: usize) -> Self {
        Self { store, window_size }
    }

    /// Persist a message to the episodic store.
    #[instrument(skip(self, message))]
    pub async fn store_message(&self, message: &Message) -> Result<()> {
        self.store
            .upsert(MESSAGE_TABLE, &message.id.to_string(), message.clone())
            .await?;
        Ok(())
    }

    /// Retrieve the most recent `window_size` messages in a session.
    pub async fn recent_messages(&self, session_id: Uuid) -> Result<Vec<Message>> {
        let sql = format!(
            "SELECT * FROM {} WHERE session_id = $sid ORDER BY timestamp DESC LIMIT $lim;",
            MESSAGE_TABLE
        );
        let mut messages: Vec<Message> = self
            .store
            .query_with(
                &sql,
                serde_json::json!({
                    "sid": session_id.to_string(),
                    "lim": self.window_size,
                }),
            )
            .await?;
        // Return in chronological order (oldest first)
        messages.reverse();
        Ok(messages)
    }

    /// Retrieve all messages in a session (used by the consolidator).
    pub async fn all_messages_in_session(&self, session_id: Uuid) -> Result<Vec<Message>> {
        let sql = format!(
            "SELECT * FROM {} WHERE session_id = $sid ORDER BY timestamp ASC;",
            MESSAGE_TABLE
        );
        self.store
            .query_with(
                &sql,
                serde_json::json!({ "sid": session_id.to_string() }),
            )
            .await
            .map_err(Into::into)
    }

    /// Retrieve all messages across all sessions since a given timestamp
    /// (used by the background consolidator to find unprocessed messages).
    pub async fn messages_since(&self, since: chrono::DateTime<Utc>) -> Result<Vec<Message>> {
        let sql = format!(
            "SELECT * FROM {} WHERE timestamp > $since ORDER BY timestamp ASC;",
            MESSAGE_TABLE
        );
        self.store
            .query_with(
                &sql,
                serde_json::json!({ "since": since }),
            )
            .await
            .map_err(Into::into)
    }
}
