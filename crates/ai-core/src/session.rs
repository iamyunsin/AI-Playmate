//! `ChatSession` — single-turn LLM completions with injected context.
//!
//! This is intentionally thin: context injection (memories) is done at a
//! higher level by `ai-playmate-memory`.  Here we only handle the mechanics
//! of calling the provider and streaming the response.

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use tracing::{debug, instrument};

use crate::{
    error::CoreError,
    provider::{LLMConfig, LLMProviderKind},
    Result,
};

/// A reusable chat session backed by a configured LLM provider.
pub struct ChatSession {
    config: LLMConfig,
}

impl ChatSession {
    pub fn new(config: LLMConfig) -> Self {
        Self { config }
    }

    /// Send a single user message with an optional system prompt and context,
    /// and return the full assistant response as a [`String`].
    ///
    /// `context_snippets` are prepended to the system prompt to inject memories.
    #[instrument(skip(self, user_message, context_snippets))]
    pub async fn complete(
        &self,
        user_message: &str,
        system_prompt: Option<&str>,
        context_snippets: &[String],
    ) -> Result<String> {
        let mut messages: Vec<ChatCompletionRequestMessage> = Vec::new();

        // Build system message with injected context
        let mut sys_parts: Vec<String> = Vec::new();
        if let Some(sp) = system_prompt {
            sys_parts.push(sp.to_string());
        }
        if !context_snippets.is_empty() {
            sys_parts.push("\n\n--- Relevant memories ---".to_string());
            for (i, snippet) in context_snippets.iter().enumerate() {
                sys_parts.push(format!("[{}] {}", i + 1, snippet));
            }
            sys_parts.push("--- End of memories ---\n".to_string());
        }

        if !sys_parts.is_empty() {
            messages.push(
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(sys_parts.join("\n"))
                    .build()
                    .map_err(|e| CoreError::Provider(e.to_string()))?
                    .into(),
            );
        }

        messages.push(
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_message)
                .build()
                .map_err(|e| CoreError::Provider(e.to_string()))?
                .into(),
        );

        self.send(messages).await
    }

    async fn send(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<String> {
        let openai_config = self.build_openai_config();
        let client = Client::with_config(openai_config);

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.config.model)
            .messages(messages)
            .temperature(self.config.temperature)
            .build()
            .map_err(|e| CoreError::Provider(e.to_string()))?;

        debug!(model = %self.config.model, "Sending chat completion request");

        let response = client
            .chat()
            .create(request)
            .await
            .map_err(|e| CoreError::Provider(e.to_string()))?;

        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();

        Ok(content)
    }

    fn build_openai_config(&self) -> OpenAIConfig {
        let mut cfg = OpenAIConfig::default();

        if let Some(key) = &self.config.api_key {
            cfg = cfg.with_api_key(key);
        }

        // Route Anthropic / Gemini through their OpenAI-compatible endpoints
        let base_url = self.config.base_url.clone().or_else(|| {
            match self.config.provider {
                LLMProviderKind::Anthropic => {
                    Some("https://api.anthropic.com/v1".to_string())
                }
                LLMProviderKind::Gemini => Some(
                    "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
                ),
                LLMProviderKind::Ollama => {
                    Some("http://localhost:11434/v1".to_string())
                }
                _ => None,
            }
        });

        if let Some(url) = base_url {
            cfg = cfg.with_api_base(url);
        }

        cfg
    }
}
