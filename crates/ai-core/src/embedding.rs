//! Embedding providers — turn text into float vectors.
//!
//! [`EmbeddingProvider`] is the trait; [`LocalEmbedder`] is the fastembed-based
//! implementation (needs HuggingFace download); [`OllamaEmbedder`] uses a locally
//! running Ollama instance (no download required).

use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::{debug, instrument};

use crate::{CoreError, Result};

/// Trait for anything that can embed text into a fixed-size float vector.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single piece of text.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Embed a batch of texts (more efficient than calling `embed` in a loop).
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Dimensionality of the output vectors.
    fn dimension(&self) -> usize;
}

// ── OllamaEmbedder ──────────────────────────────────────────────────────────

/// Embedding via a locally running Ollama instance.
///
/// Uses the `/api/embed` endpoint.  No model download required — the model
/// must already be pulled in Ollama (e.g. `ollama pull qwen3-embedding:8b`).
pub struct OllamaEmbedder {
    client: reqwest::Client,
    base_url: String,
    model: String,
    dim: usize,
}

impl OllamaEmbedder {
    /// Create an embedder backed by Ollama.
    ///
    /// * `base_url` — Ollama base URL, e.g. `http://localhost:11434`
    /// * `model`    — model name as known to Ollama, e.g. `qwen3-embedding:8b`
    /// * `dim`      — embedding dimension reported by the model
    pub fn new(base_url: impl Into<String>, model: impl Into<String>, dim: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.into(),
            dim,
        }
    }

    async fn embed_texts(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let url = format!("{}/api/embed", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });
        let resp: serde_json::Value = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| CoreError::Embedding(e.to_string()))?
            .json()
            .await
            .map_err(|e| CoreError::Embedding(e.to_string()))?;

        let embeddings = resp["embeddings"]
            .as_array()
            .ok_or_else(|| CoreError::Embedding("missing 'embeddings' field in Ollama response".into()))?
            .iter()
            .map(|row| {
                row.as_array()
                    .ok_or_else(|| CoreError::Embedding("embedding row is not an array".into()))
                    .and_then(|arr| {
                        arr.iter()
                            .map(|v| {
                                v.as_f64()
                                    .map(|f| f as f32)
                                    .ok_or_else(|| CoreError::Embedding("non-numeric value in embedding".into()))
                            })
                            .collect::<Result<Vec<f32>>>()
                    })
            })
            .collect::<Result<Vec<Vec<f32>>>>()?;

        Ok(embeddings)
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbedder {
    #[instrument(skip(self, text))]
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut results = self.embed_texts(vec![text]).await?;
        results
            .pop()
            .ok_or_else(|| CoreError::Embedding("empty embedding result".into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        debug!(count = texts.len(), "Embedding batch via Ollama");
        self.embed_texts(texts.to_vec()).await
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

// ── LocalEmbedder ────────────────────────────────────────────────────────────

/// CPU-local embedding using `fastembed` (wraps ONNX runtime).
///
/// The model is downloaded from HuggingFace on first use and cached locally.
/// Default model: `BAAI/bge-small-en-v1.5` (384-dim, ~130 MB).
pub struct LocalEmbedder {
    model: TextEmbedding,
    dim: usize,
}

impl LocalEmbedder {
    /// Create an embedder with the default model (`BAAI/bge-small-en-v1.5`).
    pub fn new() -> Result<Self> {
        Self::with_model(EmbeddingModel::BGESmallENV15)
    }

    /// Create an embedder with an explicit fastembed model.
    pub fn with_model(model_name: EmbeddingModel) -> Result<Self> {
        let dim = model_dimension(&model_name);
        let model = TextEmbedding::try_new(InitOptions::new(model_name).with_show_download_progress(true))
            .map_err(|e| CoreError::Embedding(e.to_string()))?;
        Ok(Self { model, dim })
    }
}

#[async_trait]
impl EmbeddingProvider for LocalEmbedder {
    #[instrument(skip(self, text))]
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let results = self
            .model
            .embed(vec![text.to_string()], None)
            .map_err(|e| CoreError::Embedding(e.to_string()))?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| CoreError::Embedding("empty embedding result".into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        debug!(count = texts.len(), "Embedding batch");
        let docs: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        self.model
            .embed(docs, None)
            .map_err(|e| CoreError::Embedding(e.to_string()))
    }

    fn dimension(&self) -> usize {
        self.dim
    }
}

fn model_dimension(model: &EmbeddingModel) -> usize {
    match model {
        EmbeddingModel::BGESmallENV15 => 384,
        EmbeddingModel::BGEBaseENV15 => 768,
        EmbeddingModel::BGELargeENV15 => 1024,
        EmbeddingModel::AllMiniLML6V2 => 384,
        _ => 384, // safe default
    }
}
