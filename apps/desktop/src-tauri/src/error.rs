//! Tauri command error type — converts internal errors into JSON-serialisable
//! strings that the frontend can handle.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CommandError(pub String);

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<anyhow::Error> for CommandError {
    fn from(e: anyhow::Error) -> Self {
        Self(e.to_string())
    }
}

impl From<ai_playmate_memory::MemoryError> for CommandError {
    fn from(e: ai_playmate_memory::MemoryError) -> Self {
        Self(e.to_string())
    }
}

impl From<ai_playmate_core::CoreError> for CommandError {
    fn from(e: ai_playmate_core::CoreError) -> Self {
        Self(e.to_string())
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
