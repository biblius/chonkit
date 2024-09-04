use crate::error::ChonkitError;
use std::future::Future;

/// Operations related to embeddings and their models.
pub trait Embedder {
    /// List all available models in fastembed
    fn list_embedding_models(&self) -> Vec<String>;

    /// Get the vectors for the elements in `content`.
    /// The content passed in can be a user's query,
    /// or a chunked document.
    ///
    /// * `content`: The text to embed.
    /// * `model`: The embedding model to use.
    fn embed(
        &self,
        content: Vec<String>,
        model: &str,
    ) -> impl Future<Output = Result<Vec<Vec<f32>>, ChonkitError>> + Send;

    fn size(&self, model: &str) -> Option<u64>;
}
