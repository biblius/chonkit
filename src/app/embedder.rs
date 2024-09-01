use crate::{core::embedder::Embedder, error::ChonkitError};
use fastembed::{InitOptions, ModelInfo, TextEmbedding};

#[derive(Debug, Clone)]
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
