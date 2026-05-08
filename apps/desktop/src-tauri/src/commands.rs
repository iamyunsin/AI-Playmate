//! Tauri IPC commands — the bridge between the React frontend and Rust core.

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;
use uuid::Uuid;

use ai_playmate_memory::types::Message;

use crate::{error::CommandResult, state::AppState};

// ── Request / Response DTOs ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub session_id: String,
    pub content: String,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub reply: String,
    pub session_id: String,
    pub context_snippets_used: usize,
}

#[derive(Debug, Serialize)]
pub struct MemoryStats {
    pub estimated_memories: usize,
}

// ── Commands ─────────────────────────────────────────────────────────────────

/// Send a user message and get an AI reply, with associative memory context.
#[tauri::command]
pub async fn send_message(
    request: SendMessageRequest,
    state: State<'_, Mutex<AppState>>,
) -> CommandResult<SendMessageResponse> {
    let state = state.lock().await;

    let session_id = Uuid::parse_str(&request.session_id)
        .unwrap_or_else(|_| Uuid::new_v4());

    // 1. Persist the user message
    let user_msg = Message::user(session_id, &request.content);
    state.memory.store_message(&user_msg).await?;

    // 2. Retrieve associative context
    let context = state
        .memory
        .retrieve_context(&request.content, session_id)
        .await?;
    let snippets = context.to_snippets();
    let snippet_count = snippets.len();

    // 3. Core memory as system prompt prefix
    let core_prompt = state
        .memory
        .core
        .to_prompt_string()
        .await
        .unwrap_or_default();
    let system = format!(
        "{}\n\n{}",
        core_prompt,
        request.system_prompt.unwrap_or_default()
    );

    // 4. Call the LLM
    let reply = state
        .chat_session
        .complete(&request.content, Some(&system), &snippets)
        .await?;

    // 5. Persist the assistant reply
    let assistant_msg = Message::assistant(session_id, &reply);
    state.memory.store_message(&assistant_msg).await?;

    Ok(SendMessageResponse {
        reply,
        session_id: session_id.to_string(),
        context_snippets_used: snippet_count,
    })
}

/// Return a human-readable snapshot of the core (always-in-context) memory.
#[tauri::command]
pub async fn get_core_memory(
    state: State<'_, Mutex<AppState>>,
) -> CommandResult<String> {
    let state = state.lock().await;
    let prompt = state
        .memory
        .core
        .to_prompt_string()
        .await
        .map_err(|e| crate::error::CommandError(e.to_string()))?;
    Ok(prompt)
}

/// List recent messages for a session.
#[tauri::command]
pub async fn get_recent_messages(
    session_id: String,
    state: State<'_, Mutex<AppState>>,
) -> CommandResult<Vec<serde_json::Value>> {
    let state = state.lock().await;
    let session_id = Uuid::parse_str(&session_id).unwrap_or_else(|_| Uuid::new_v4());
    let messages = state
        .memory
        .episodic
        .recent_messages(session_id)
        .await?;
    let json = messages
        .into_iter()
        .map(|m| serde_json::to_value(m).unwrap_or_default())
        .collect();
    Ok(json)
}

/// Search semantic memories by query text.
#[tauri::command]
pub async fn search_memories(
    query: String,
    top_k: Option<usize>,
    state: State<'_, Mutex<AppState>>,
) -> CommandResult<Vec<serde_json::Value>> {
    let state = state.lock().await;
    let memories = state.memory.semantic.search(&query, top_k).await?;
    let json = memories
        .into_iter()
        .map(|m| serde_json::to_value(m).unwrap_or_default())
        .collect();
    Ok(json)
}

/// Update the user profile section of core memory.
#[tauri::command]
pub async fn update_user_profile(
    profile: String,
    state: State<'_, Mutex<AppState>>,
) -> CommandResult<()> {
    let state = state.lock().await;
    let mut record = state.memory.core.get().await?;
    record.user_profile = profile;
    state.memory.core.update(record).await?;
    Ok(())
}
