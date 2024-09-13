use crate::error::ChonkitError;

/// Operations related to embeddings and their models.
#[async_trait::async_trait]
pub trait Embedder {
    /// Return the embedder's identifier.
    fn id(&self) -> &'static str;

    /// Used for creating the initial collection.
    fn default_model(&self) -> (String, usize);

    /// List all available models in the embedder and their sizes.
    fn list_embedding_models(&self) -> Vec<(String, usize)>;

    /// Return the size of the given model's embeddings
    /// if it is supported by the embedder.
    ///
    /// * `model`:
    fn size(&self, model: &str) -> Option<usize>;

    /// Get the vectors for the elements in `content`.
    /// The content passed in can be a user's query,
    /// or a chunked document.
    ///
    /// * `content`: The text to embed.
    /// * `model`: The embedding model to use.
    async fn embed(&self, content: &[&str], model: &str) -> Result<Vec<Vec<f32>>, ChonkitError>;
}
