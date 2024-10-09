use crate::error::ChonkitError;

/// Operations related to embeddings and their models.
#[async_trait::async_trait]
pub trait Embedder {
    /// Return the embedder's identifier.
    fn id(&self) -> &'static str;

    /// Used for creating the initial collection.
    fn default_model(&self) -> (String, usize);

    /// List all available models in the embedder and their sizes.
    async fn list_embedding_models(&self) -> Result<Vec<(String, usize)>, ChonkitError>;

    /// Return the size of the given model's embeddings
    /// if it is supported by the embedder.
    ///
    /// * `model`:
    async fn size(&self, model: &str) -> Result<Option<usize>, ChonkitError> {
        Ok(self
            .list_embedding_models()
            .await?
            .into_iter()
            .find(|m| m.0 == model)
            .map(|m| m.1))
    }

    /// Get the vectors for the elements in `content`.
    /// The content passed in can be a user's query,
    /// or a chunked document.
    ///
    /// * `content`: The text to embed.
    /// * `model`: The embedding model to use.
    async fn embed(&self, content: &[&str], model: &str) -> Result<Vec<Vec<f32>>, ChonkitError>;
}
