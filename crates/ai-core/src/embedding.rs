//! Embedding providers — turn text into float vectors.
//!
//! [`EmbeddingProvider`] is the trait; [`LocalEmbedder`] is the default
//! implementation using `fastembed` (CPU inference, no external service).

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
