use std::collections::HashMap;

use super::{DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_SIZE};
use crate::{core::embedder::Embedder, error::ChonkitError};
use serde::{Deserialize, Serialize};

pub struct FastEmbedder {
    pub client: reqwest::Client,
    pub url: String,
}

impl FastEmbedder {
    /// Initialise the FastEmbedder remote client.
    pub fn new(url: String) -> FastEmbedder {
        tracing::info!("Initializing remote Fastembed at {url}");
        let client = reqwest::Client::new();
        FastEmbedder { client, url }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.url)
    }
}

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

    async fn list_embedding_models(&self) -> Result<Vec<(String, usize)>, ChonkitError> {
        let url = self.url("list");
        let response: HashMap<String, usize> = self.client.get(&url).send().await?.json().await?;

        Ok(response
            .into_iter()
            .map(|(model, size)| (model, size))
            .collect())
    }

    async fn embed(&self, content: &[&str], model: &str) -> Result<Vec<Vec<f64>>, ChonkitError> {
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
    embeddings: Vec<Vec<f64>>,
}

impl std::fmt::Debug for FastEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastEmbedder").finish()
    }
}
