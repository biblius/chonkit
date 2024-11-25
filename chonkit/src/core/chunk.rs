use super::embedder::Embedder;
use crate::error::ChonkitError;
use chunx::ChunkerError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChunkConfig {
    Sliding(SlidingWindowConfig),
    Snapping(SnappingWindowConfig),
    Semantic(SemanticWindowConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SlidingWindowConfig {
    pub size: usize,
    pub overlap: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SnappingWindowConfig {
    pub size: usize,
    pub overlap: usize,
    pub delimiter: char,
    pub skip_f: Vec<String>,
    pub skip_b: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SemanticWindowConfig {
    pub size: usize,
    pub threshold: f64,
    pub distance_fn: chunx::semantic::DistanceFn,
    pub delimiter: char,
    pub skip_f: Vec<String>,
    pub skip_b: Vec<String>,
    pub embedding_model: String,
    pub embedding_provider: String,
}

impl ChunkConfig {
    /// Create a `SlidingWindow` chunker.
    ///
    /// * `size`: Chunk base size.
    /// * `overlap`: Chunk overlap.
    pub fn sliding(size: usize, overlap: usize) -> Result<Self, ChunkerError> {
        Ok(Self::Sliding(SlidingWindowConfig { size, overlap }))
    }

    /// Create a default `SlidingWindow` chunker.
    pub fn sliding_default() -> Self {
        let config = chunx::SlidingWindow::default();
        let config = SlidingWindowConfig {
            size: config.size,
            overlap: config.overlap,
        };
        Self::Sliding(config)
    }

    /// Create a `SnappingWindow` chunker.
    ///
    /// * `size`: Chunk base size.
    /// * `overlap`: Chunk overlap.
    /// * `skip_f`: Patterns in front of delimiters to not treat as sentence stops.
    /// * `skip_b`: Patterns behind delimiters to not treat as sentence stops.
    pub fn snapping(
        size: usize,
        overlap: usize,
        skip_f: Vec<String>,
        skip_b: Vec<String>,
    ) -> Result<Self, ChunkerError> {
        Ok(Self::Snapping(SnappingWindowConfig {
            size,
            overlap,
            skip_f,
            skip_b,
            delimiter: '.',
        }))
    }

    /// Create a default `SnappingWindow` chunker.
    pub fn snapping_default() -> Self {
        let config = chunx::SnappingWindow::default();
        let config = SnappingWindowConfig {
            size: config.size,
            overlap: config.overlap,
            skip_f: config.skip_forward,
            skip_b: config.skip_back,
            delimiter: '.',
        };
        Self::Snapping(config)
    }

    /// Create a `SemanticWindow` chunker.
    ///
    /// * `size`: Amount of sentences per chunk.
    /// * `threshold`: Threshold for semantic similarity.
    /// * `distance_fn`: Distance function to use for semantic similarity.
    /// * `embedder`: Embedder to use for embedding chunks.
    /// * `model`: Model to use for embeddings.
    pub fn semantic(
        size: usize,
        threshold: f64,
        delimiter: char,
        distance_fn: chunx::semantic::DistanceFn,
        embedding_provider: String,
        embedding_model: String,
        skip_f: Vec<String>,
        skip_b: Vec<String>,
    ) -> Self {
        Self::Semantic(SemanticWindowConfig {
            size,
            threshold,
            distance_fn,
            delimiter,
            embedding_provider,
            embedding_model,
            skip_f,
            skip_b,
        })
    }

    /// Create a default `SemanticWindow` chunker.
    ///
    /// * `embedder`: Embedder to use for embedding chunks, uses the default embedder model.
    pub fn semantic_default(embedding_provider: String, embedding_model: String) -> Self {
        let config = chunx::semantic::SemanticWindow::default();
        let config = SemanticWindowConfig {
            size: config.size,
            delimiter: config.delimiter,
            distance_fn: config.distance_fn,
            threshold: config.threshold,
            skip_f: config.skip_forward,
            skip_b: config.skip_back,
            embedding_provider,
            embedding_model,
        };
        Self::Semantic(config)
    }
}

impl std::fmt::Display for ChunkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sliding(_) => write!(f, "SlidingWindow"),
            Self::Snapping(_) => write!(f, "SnappingWindow"),
            Self::Semantic(_) => write!(f, "SemanticWindow"),
        }
    }
}

/// The result of chunking a document.
/// Some chunkers do not allocate.
pub enum ChunkedDocument<'content> {
    Ref(Vec<&'content str>),
    Owned(Vec<String>),
}

pub struct SemanticEmbedder(pub std::sync::Arc<dyn Embedder + Send + Sync>);

impl chunx::semantic::Embedder for SemanticEmbedder {
    type Error = ChonkitError;

    async fn embed(&self, input: &[&str], model: &str) -> Result<Vec<Vec<f64>>, Self::Error> {
        let embeddings = self.0.embed(input, model).await?;
        Ok(embeddings)
    }
}
