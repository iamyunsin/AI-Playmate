//! LLM provider configuration.
//!
//! All configuration is read from environment variables so credentials are
//! never hard-coded.  See `.env.example` at the repository root.

use serde::{Deserialize, Serialize};

/// Which LLM backend to use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LLMProviderKind {
    /// OpenAI or any compatible API (Ollama, Azure, local proxy …).
    OpenAI,
    /// Anthropic Claude models via the official Messages API.
    Anthropic,
    /// Google Gemini models.
    Gemini,
    /// Ollama running locally — treated as an OpenAI-compatible endpoint.
    Ollama,
}

/// Full LLM configuration, typically built via [`LLMConfig::from_env`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: LLMProviderKind,
    pub api_key: Option<String>,
    /// Override the base URL (useful for Ollama / Azure / proxies).
    pub base_url: Option<String>,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: f32,
}

impl LLMConfig {
    /// Build configuration from environment variables.
    ///
    /// Priority: OpenAI → Anthropic → Gemini → Ollama (fallback, no key needed).
    pub fn from_env() -> crate::Result<Self> {
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            return Ok(Self {
                provider: LLMProviderKind::OpenAI,
                api_key: Some(api_key),
                base_url: std::env::var("OPENAI_BASE_URL").ok(),
                model: std::env::var("OPENAI_MODEL")
                    .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
                max_tokens: None,
                temperature: 0.7,
            });
        }

        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            return Ok(Self {
                provider: LLMProviderKind::Anthropic,
                api_key: Some(api_key),
                base_url: None,
                model: std::env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-3-5-haiku-20241022".to_string()),
                max_tokens: None,
                temperature: 0.7,
            });
        }

        if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
            return Ok(Self {
                provider: LLMProviderKind::Gemini,
                api_key: Some(api_key),
                base_url: None,
                model: std::env::var("GEMINI_MODEL")
                    .unwrap_or_else(|_| "gemini-2.0-flash".to_string()),
                max_tokens: None,
                temperature: 0.7,
            });
        }

        // Fallback: local Ollama
        Ok(Self {
            provider: LLMProviderKind::Ollama,
            api_key: None,
            base_url: Some(
                std::env::var("OLLAMA_BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            ),
            model: std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "llama3.2".to_string()),
            max_tokens: None,
            temperature: 0.7,
        })
    }
}
