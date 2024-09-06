use crate::error::ChonkitError;
use std::future::Future;

/// Operations related to embeddings and their models.
pub trait Embedder {
    /// Return the embedder's identifier.
    fn id(&self) -> &'static str;

    /// List all available models in the embedder and their sizes.
    fn list_embedding_models(&self) -> Vec<(String, usize)>;

    /// Get the vectors for the elements in `content`.
    /// The content passed in can be a user's query,
    /// or a chunked document.
    ///
    /// * `content`: The text to embed.
    /// * `model`: The embedding model to use.
    fn embed(
        &self,
        content: &[String],
        model: &str,
    ) -> impl Future<Output = Result<Vec<Vec<f32>>, ChonkitError>> + Send;

    /// Return the size of the given model's embeddings
    /// if it is supported by the embedder.
    ///
    /// * `model`:
    fn size(&self, model: &str) -> Option<usize>;
}
