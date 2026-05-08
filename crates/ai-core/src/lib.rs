//! `ai-playmate-core` — AI provider abstractions for AI Playmate.
//!
//! Provides:
//! * [`LLMConfig`] — unified configuration loaded from environment variables.
//! * [`LLMProvider`] — enum covering OpenAI-compatible, Anthropic and Ollama.
//! * [`EmbeddingProvider`] — trait for embedding text into float vectors.
//! * [`LocalEmbedder`] — CPU-local embedding via `fastembed`.
//! * [`ChatSession`] — thin wrapper around `rig` agents for stateless completions.

pub mod embedding;
pub mod error;
pub mod provider;
pub mod session;

pub use embedding::{EmbeddingProvider, LocalEmbedder, OllamaEmbedder};
pub use error::CoreError;
pub use provider::{LLMConfig, LLMProviderKind};
pub use session::ChatSession;

pub type Result<T> = std::result::Result<T, CoreError>;
