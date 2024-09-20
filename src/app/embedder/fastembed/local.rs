//! Embedder implementation for fastembed when running it
//! locally.

use super::{list_models, FastEmbedder, DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_SIZE};
use crate::{core::embedder::Embedder, error::ChonkitError};

#[async_trait::async_trait]
impl Embedder for FastEmbedder {
    fn id(&self) -> &'static str {
        "fembed"
    }

    fn default_model(&self) -> (String, usize) {
        (
            String::from(DEFAULT_COLLECTION_MODEL),
            DEFAULT_COLLECTION_SIZE,
        )
    }

    fn list_embedding_models(&self) -> Vec<(String, usize)> {
        list_models()
            .into_iter()
            .map(|model| (model.model_code, model.dim))
            .collect()
    }

    async fn embed(&self, content: &[&str], model: &str) -> Result<Vec<Vec<f32>>, ChonkitError> {
        let embedder = self.models.get(model).ok_or_else(|| {
            ChonkitError::InvalidEmbeddingModel(format!(
                "Model '{model}' not supported by embedder '{}'",
                self.id()
            ))
        })?;

        let embeddings = embedder
            .embed(content.to_vec(), None)
            .map_err(|err| ChonkitError::Fastembed(err.to_string()))?;

        debug_assert_eq!(
            embeddings.len(),
            content.len(),
            "Content length is different from embeddings!"
        );

        Ok(embeddings)
    }
}
