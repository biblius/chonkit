use crate::error::EmbeddingError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Embedder implementation for communicating with a feserver on a remote
/// machine supporting CUDA.
pub struct RemoteFastEmbedder {
    pub client: reqwest::Client,
    pub url: String,
}

impl RemoteFastEmbedder {
    /// Initialise the FastEmbedder remote client.
    pub fn new(url: String) -> RemoteFastEmbedder {
        tracing::info!("Initializing remote Fastembed at {url}");
        let client = reqwest::Client::new();
        RemoteFastEmbedder { client, url }
    }
    pub async fn list_models(&self) -> Result<Vec<(String, usize)>, EmbeddingError> {
        let url = self.url("list");
        let response: HashMap<String, usize> = self.client.get(&url).send().await?.json().await?;
        Ok(response.into_iter().collect())
    }

    pub async fn embed(
        &self,
        content: &[&str],
        model: &str,
    ) -> Result<Vec<Vec<f64>>, EmbeddingError> {
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

    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.url)
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

impl std::fmt::Debug for RemoteFastEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastEmbedder")
            .field("url", &self.url)
            .finish()
    }
}
