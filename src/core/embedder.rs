use std::future::Future;

use fastembed::{EmbeddingModel, InitOptions, ModelInfo, TextEmbedding};

use crate::error::ChonkitError;

/// # CORE
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
        content: Vec<&str>,
        model: &str,
    ) -> impl Future<Output = Result<Vec<Vec<f32>>, ChonkitError>> + Send;

    fn size(&self, model: &str) -> Option<u64>;
}

#[derive(Debug, Clone, Copy)]
pub struct FastEmbedder;

impl Embedder for FastEmbedder {
    fn list_embedding_models(&self) -> Vec<String> {
        fastembed::TextEmbedding::list_supported_models()
            .into_iter()
            .map(|m| m.model_code)
            .collect()
    }

    async fn embed(&self, content: Vec<&str>, model: &str) -> Result<Vec<Vec<f32>>, ChonkitError> {
        let model = self.model_for_str(model).ok_or_else(|| {
            ChonkitError::UnsupportedEmbeddingModel(format!(
                "{model} is not a valid fastembed model",
            ))
        })?;

        let embedder = TextEmbedding::try_new(InitOptions {
            model_name: model.model,
            show_download_progress: true,
            ..Default::default()
        })
        .map_err(|err| ChonkitError::Fastembed(err.to_string()))?;

        let embeddings = embedder
            .embed(content.clone(), None)
            .map_err(|err| ChonkitError::Fastembed(err.to_string()))?;

        debug_assert_eq!(
            embeddings.len(),
            content.len(),
            "Content length is different from embeddings!"
        );

        Ok(embeddings)
    }

    fn size(&self, model: &str) -> Option<u64> {
        self.model_for_str(model).map(|m| m.dim as u64)
    }
}

impl FastEmbedder {
    fn model_for_str(&self, s: &str) -> Option<ModelInfo> {
        fastembed::TextEmbedding::list_supported_models()
            .into_iter()
            .find(|model| model.model_code == s)
    }
}
