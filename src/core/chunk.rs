use super::embedder::Embedder;
use serde::{Deserialize, Serialize};
use std::{future::Future, str::Utf8Error, sync::Arc};
use thiserror::Error;

mod cursor;
mod semantic;
mod sliding;
mod snapping;

pub use semantic::{DistanceFn, SemanticWindow, SemanticWindowConfig};
pub use sliding::SlidingWindow;
pub use snapping::SnappingWindow;

#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Chunker {
    Sliding(SlidingWindow),
    Snapping(SnappingWindow),
    Semantic(SemanticWindow),
}

impl Chunker {
    /// Create a `SlidingWindow` chunker.
    ///
    /// * `size`: Chunk base size.
    /// * `overlap`: Chunk overlap.
    pub fn sliding(size: usize, overlap: usize) -> Self {
        Self::Sliding(SlidingWindow::new(size, overlap))
    }

    /// Create a default `SlidingWindow` chunker.
    pub fn sliding_default() -> Self {
        Self::Sliding(SlidingWindow::default())
    }

    /// Create a `SnappingWindow` chunker.
    ///
    /// * `size`: Chunk base size.
    /// * `overlap`: Chunk overlap.
    /// * `skip_f`: Patterns in front of delimiters to not treat as sentence stops.
    /// * `skip_b`: Patterns behind delimiters to not treat as sentence stops.
    pub fn snapping(size: usize, overlap: usize, skip_f: Vec<String>, skip_b: Vec<String>) -> Self {
        Self::Snapping(
            SnappingWindow::new(size, overlap)
                .skip_forward(skip_f)
                .skip_back(skip_b),
        )
    }

    /// Create a default `SnappingWindow` chunker.
    pub fn snapping_default() -> Self {
        Self::Snapping(SnappingWindow::default())
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
        distance_fn: DistanceFn,
        embedder: Arc<dyn Embedder + Send + Sync>,
        model: String,
    ) -> Self {
        Self::Semantic(SemanticWindow::new(
            size,
            threshold,
            distance_fn,
            embedder,
            model,
        ))
    }

    /// Create a default `SemanticWindow` chunker for the embedder.
    ///
    /// * `embedder`: Embedder to use for embedding chunks, uses the default embedder model.
    pub fn semantic_default(embedder: Arc<dyn Embedder + Send + Sync>) -> Self {
        Self::Semantic(SemanticWindow::default(embedder))
    }

    /// Chunk the input using the current variant.
    ///
    /// * `input`: Input to chunk.
    pub async fn chunk<'content>(
        &self,
        input: &'content str,
    ) -> Result<ChunkedDocument<'content>, ChunkerError> {
        match self {
            Self::Sliding(chunker) => Ok(ChunkedDocument::Ref(chunker.chunk(input).await?)),
            Self::Snapping(chunker) => Ok(ChunkedDocument::Ref(chunker.chunk(input).await?)),
            Self::Semantic(chunker) => Ok(ChunkedDocument::Owned(chunker.chunk(input).await?)),
        }
    }
}

impl std::fmt::Display for Chunker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sliding(_) => write!(f, "SlidingWindow"),
            Self::Snapping(_) => write!(f, "SnappingWindow"),
            Self::Semantic(_) => write!(f, "SemanticWindow"),
        }
    }
}

pub enum ChunkedDocument<'content> {
    Ref(Vec<&'content str>),
    Owned(Vec<String>),
}

pub trait DocumentChunker<'a> {
    type Output: AsRef<str> + 'a;

    fn chunk(
        &self,
        input: &'a str,
    ) -> impl Future<Output = Result<Vec<Self::Output>, ChunkerError>> + Send;
}

#[derive(Debug, Error)]
pub enum ChunkerError {
    #[error("{0}")]
    Config(String),

    #[error("utf-8: {0}")]
    Utf8(#[from] Utf8Error),

    #[error("error in semantic chunker embedder: {0}")]
    Embedder(String),
}

#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkBaseConfig {
    /// Base chunk size.
    pub size: usize,

    /// The overlap per chunk.
    pub overlap: usize,
}

impl ChunkBaseConfig {
    pub fn new(size: usize, overlap: usize) -> Self {
        Self { size, overlap }
    }
}

#[inline(always)]
fn concat<'a>(start_str: &'a str, end_str: &'a str) -> Result<&'a str, Utf8Error> {
    let current_ptr =
        std::ptr::slice_from_raw_parts(start_str.as_ptr(), start_str.len() + end_str.len());
    unsafe { std::str::from_utf8(&*current_ptr) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_sanity() {
        let input = "Hello\nWorld";
        let split = input.split_inclusive('\n').collect::<Vec<_>>();

        let one = split[0];
        let two = split[1];

        assert_eq!(input, concat(one, two).unwrap())
    }
}
