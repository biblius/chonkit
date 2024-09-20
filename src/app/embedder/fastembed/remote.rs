//! Embedder implementation for running fastembed on a remote
//! machine supporting CUDA.

use super::{list_models, FastEmbedder, DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_SIZE};
use crate::{core::embedder::Embedder, error::ChonkitError};
use serde::{Deserialize, Serialize};

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
        let url = self.url("embed");
        let request = EmbedRequest {
            model: model.to_string(),
            content: content.iter().map(|s| s.to_string()).collect(),
        };

        let response: EmbedResponse = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        Ok(response.embeddings)
    }
}

#[derive(Debug, Serialize)]
pub struct EmbedRequest {
    model: String,
    content: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}
