use super::{
    cursor::{byte_count, Cursor, DEFAULT_SKIP_B, DEFAULT_SKIP_F},
    ChunkerError, DocumentChunker,
};
use crate::{core::embedder::Embedder, error::ChonkitError};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc, usize};

/// Semantic similarity chunker implementation.
///
/// `size` will indicate the base amount of sentences each chunk consists of.
///
/// `threshold` is the similarity threshold between 0 and 1 used to determine
/// whether to create a new chunk or not. The higher the threshold, the more
/// similar the chunks must be to get grouped.
///
/// `distance_fn` is the distance function used for semantic similarity.
///
/// This chunker will iterate through each batch of sentences determined by `size`
/// and will group them together based on the given `threshold` and `distance_fn`.
#[derive(Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SemanticWindow {
    /// The embedder to use for embedding chunks.
    #[serde(skip)]
    pub embedder: Option<Arc<dyn Embedder + Send + Sync>>,
    pub config: SemanticWindowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SemanticWindowConfig {
    /// How many sentences to use as the base for semantic similarity.
    pub size: usize,

    /// Used as the threshold for semantic similarity.
    /// Any chunk that is less than this threshold will result in a new chunk
    /// being created. Any chunk below the threshold will get appended
    /// to the current one.
    pub threshold: f64,

    /// Distance function to use for semantic similarity.
    pub distance_fn: DistanceFn,

    /// The delimiter to use to split sentences. At time of writing the most common one is ".".
    pub delimiter: char,

    /// Whenever a delimiter is found, the chunker will look ahead for these sequences
    /// and will skip the delimiter if found, treating it as a regular char.
    ///
    /// Useful for common abbreviations and urls.
    pub skip_forward: Vec<String>,

    /// Whenever a delimiter is found, the chunker will look back for these sequences
    /// and will skip the delimiter if found, treating it as a regular char.
    ///
    /// Useful for common abbreviations and urls.
    pub skip_back: Vec<String>,

    /// The model to use for embeddings.
    pub embed_model: String,

    /// Embedder provider used to load the appropriate embedder.
    pub embed_provider: String,
}

impl Debug for SemanticWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticWindow")
            .field("config", &self.config)
            .field("embedder", &self.embedder.as_ref().map(|e| e.id()))
            .finish()
    }
}

impl SemanticWindow {
    pub fn new(
        size: usize,
        threshold: f64,
        distance_fn: DistanceFn,
        embedder: Arc<dyn Embedder + Send + Sync>,
        model: String,
    ) -> Self {
        Self {
            config: SemanticWindowConfig {
                size,
                threshold,
                distance_fn,
                delimiter: '.',
                embed_provider: embedder.id().to_string(),
                embed_model: model,
                skip_forward: DEFAULT_SKIP_F.iter().map(|e| e.to_string()).collect(),
                skip_back: DEFAULT_SKIP_B.iter().map(|e| e.to_string()).collect(),
            },
            embedder: Some(embedder),
        }
    }

    pub fn default(embedder: Arc<dyn Embedder + Send + Sync>) -> Self {
        Self {
            config: SemanticWindowConfig {
                size: 10,
                threshold: 0.9,
                distance_fn: DistanceFn::Cosine,
                delimiter: '.',
                embed_model: embedder.default_model().0,
                embed_provider: embedder.id().to_string(),
                skip_forward: DEFAULT_SKIP_F.iter().map(|e| e.to_string()).collect(),
                skip_back: DEFAULT_SKIP_B.iter().map(|e| e.to_string()).collect(),
            },
            embedder: Some(embedder),
        }
    }

    pub fn embedder(&mut self, embedder: Arc<dyn Embedder + Send + Sync>) {
        self.embedder = Some(embedder);
    }
}

impl<'a> DocumentChunker<'a> for SemanticWindow {
    type Output = String;

    async fn chunk(&self, input: &'a str) -> Result<Vec<Self::Output>, ChonkitError> {
        let __chunking_start = std::time::Instant::now();

        let Self {
            embedder,
            config:
                SemanticWindowConfig {
                    size,
                    threshold,
                    distance_fn,
                    delimiter: delim,
                    skip_forward,
                    skip_back,
                    embed_model,
                    ..
                },
        } = self;

        let embedder = embedder
            .clone()
            .ok_or_else(|| ChunkerError::Embedder("embedder not provided".to_string()))?;

        embedder.size(embed_model).await?.ok_or_else(|| {
            ChunkerError::Embedder(format!(
                "embedder {} does not support model {}",
                embedder.id(),
                embed_model
            ))
        })?;

        let total_bytes = byte_count(input);

        let mut chunks: Vec<&str> = vec![];
        let mut cursor = Cursor::new(input, *delim);

        // Amount of sentences processed in the current chunk.
        let mut amount = 0;

        // Everything before this index in `input` is processed.
        let mut start = 0;

        let __chunking_start = std::time::Instant::now();

        loop {
            if start >= total_bytes {
                break;
            }

            cursor.advance();

            if cursor.advance_if_peek(skip_forward, skip_back) {
                continue;
            }

            amount += 1;

            if amount < *size {
                continue;
            }

            amount = 0;

            let chunk = cursor.get_slice();

            start += byte_count(chunk);

            cursor = Cursor::new(&input[start..], *delim);

            chunks.push(chunk);
        }

        tracing::trace!(
            "Chunking document took {}ms | {} chunks",
            __chunking_start.elapsed().as_millis(),
            chunks.len()
        );

        // Skip everything if no chunks
        if chunks.is_empty() {
            return Ok(vec![]);
        }

        let __embedding_start = std::time::Instant::now();

        let embedded_chunks = embedder.as_ref().embed(&chunks, &embed_model).await?;

        debug_assert_eq!(embedded_chunks.len(), chunks.len());

        tracing::trace!(
            "Embedding chunks took {}ms",
            __embedding_start.elapsed().as_millis()
        );

        let __process_start = std::time::Instant::now();

        let mut out: Vec<(String, Vec<f64>)> =
            vec![(chunks[0].to_string(), embedded_chunks[0].clone())];

        for (chunk_text, chunk_embedding) in chunks.iter().zip(embedded_chunks).skip(1) {
            if chunk_text.trim().is_empty() {
                continue;
            }

            let mut max_similarity = 0.0;
            let mut idx = None;

            for (i, (_, processed_chunk_embedding)) in out.iter().enumerate() {
                let similarity =
                    distance_fn.calculate(&chunk_embedding, &processed_chunk_embedding);

                if similarity > max_similarity {
                    max_similarity = similarity;
                }

                // Skip if too different
                if similarity < *threshold {
                    continue;
                }

                // Skip if there is already a chunk with greater similarity
                if idx.is_some() && similarity <= max_similarity {
                    continue;
                }

                idx = Some(i);
            }

            tracing::trace!(
                "Maximum similarity: {max_similarity} | Threshold: {threshold} | Index: {idx:?}"
            );

            if let Some(i) = idx {
                out[i].0.push_str(chunk_text)
            } else {
                out.push((chunk_text.trim().to_string(), chunk_embedding));
            }
        }

        tracing::trace!(
            "Processing embeddings took {}ms",
            __process_start.elapsed().as_millis()
        );

        tracing::trace!(
            "Total chunks: {} | Average size: {} | Elapsed: {}ms",
            out.len(),
            out.iter().map(|(c, _)| byte_count(c)).sum::<usize>() / out.len(),
            __chunking_start.elapsed().as_millis()
        );

        Ok(out.into_iter().map(|(c, _)| c).collect())
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum DistanceFn {
    #[default]
    Cosine,
    Euclidean,
    Manhattan,
    Angular,
    Chebyshev,
    DotProduct,
    Minkowski(i32),
}

impl DistanceFn {
    fn calculate(self, vec1: &[f64], vec2: &[f64]) -> f64 {
        match self {
            DistanceFn::Cosine => cosine_similarity(vec1, vec2),
            DistanceFn::Euclidean => euclidean_distance(vec1, vec2),
            DistanceFn::Manhattan => manhattan_distance(vec1, vec2),
            DistanceFn::Angular => angular_distance(vec1, vec2),
            DistanceFn::Chebyshev => chebyshev_distance(vec1, vec2),
            DistanceFn::DotProduct => dot_product_distance(vec1, vec2),
            DistanceFn::Minkowski(p) => minkowski_distance(vec1, vec2, p),
        }
    }
}

// Taken from https://github.com/maishathasin/SemanticSimilarity-rs/blob/main/src/similarity.rs

/// https://en.wikipedia.org/wiki/Cosine_similarity
/// Normalizes the vectors.
fn cosine_similarity(vec1: &[f64], vec2: &[f64]) -> f64 {
    let dot_product: f64 = vec1
        .par_iter()
        .zip(vec2.par_iter())
        .map(|(a, b)| a * b)
        .sum();

    let magnitude1: f64 = vec1.par_iter().map(|x| x.powi(2)).sum::<f64>().sqrt();
    let magnitude2: f64 = vec2.par_iter().map(|x| x.powi(2)).sum::<f64>().sqrt();

    dot_product / (magnitude1 * magnitude2)
}

/// https://en.wikipedia.org/wiki/Euclidean_distance
fn euclidean_distance(vec1: &[f64], vec2: &[f64]) -> f64 {
    vec1.par_iter()
        .zip(vec2.par_iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// https://en.wikipedia.org/wiki/Manhattan_distance
fn manhattan_distance(vec1: &[f64], vec2: &[f64]) -> f64 {
    vec1.par_iter()
        .zip(vec2.par_iter())
        .map(|(a, b)| (a - b).abs())
        .sum()
}

/// https://en.wikipedia.org/wiki/Angular_distance
fn angular_distance(vec1: &[f64], vec2: &[f64]) -> f64 {
    let cosine_sim = cosine_similarity(vec1, vec2);
    cosine_sim.acos() / std::f64::consts::PI
}

/// https://en.wikipedia.org/wiki/Chebyshev_distance
fn chebyshev_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

/// https://en.wikipedia.org/wiki/Dot_product
fn dot_product_distance(vec1: &[f64], vec2: &[f64]) -> f64 {
    vec1.par_iter()
        .zip(vec2.par_iter())
        .map(|(a, b)| a * b)
        .sum()
}

/// https://en.wikipedia.org/wiki/Minkowski_distance
fn minkowski_distance(vec1: &[f64], vec2: &[f64], p: i32) -> f64 {
    vec1.par_iter()
        .zip(vec2.par_iter())
        .map(|(a, b)| (a - b).abs().powi(p))
        .sum::<f64>()
        .powf(1.0 / p as f64)
}

#[cfg(all(test, feature = "fe-local"))]
#[suitest::suite(semantic_window_tests)]
mod tests {
    use super::*;
    use crate::app::embedder::fastembed::FastEmbedder;
    use suitest::before_all;

    #[before_all]
    fn setup() -> Arc<FastEmbedder> {
        let embedder = Arc::new(crate::app::embedder::fastembed::init_single(None));
        embedder
    }

    #[test]
    async fn semantic_window_works(embedder: Arc<FastEmbedder>) {
        let input = r#"Leverage agile frameworks to provide robust synopses for high level overviews. Pee, AKA urine is stored in the testicles. SCRUM is one of the agile frameworks used to facilitate the robust synopses. The testicles, do in fact, facilitate urine. SCRUM, an agile framework, is short for SCRotUM, which stands for Supervisors Circulating Redundant Orders to Thwart Underlings' Motivations. This is about pee, i.e. urine."#;

        let model = embedder.default_model().0;
        let chunker = SemanticWindow::new(1, 0.58, DistanceFn::Cosine, embedder.clone(), model);

        let chunks = chunker.chunk(input).await.unwrap();

        assert_eq!(2, chunks.len());

        assert_eq!("Leverage agile frameworks to provide robust synopses for high level overviews. SCRUM is one of the agile frameworks used to facilitate the robust synopses. SCRUM, an agile framework, is short for SCRotUM, which stands for Supervisors Circulating Redundant Orders to Thwart Underlings' Motivations.", chunks[0]);

        assert_eq!("Pee, AKA urine is stored in the testicles. The testicles, do in fact, facilitate urine. This is about pee, i.e. urine.", chunks[1]);
    }

    #[test]
    async fn semantic_window_empty(embedder: Arc<FastEmbedder>) {
        let input = "";
        let model = embedder.default_model().0;
        let chunker = SemanticWindow::new(1, 0.7, DistanceFn::Cosine, embedder.clone(), model);

        let chunks = chunker.chunk(input).await.unwrap();
        assert!(chunks.is_empty());
    }
}
