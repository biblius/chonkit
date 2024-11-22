use crate::error::EmbeddingError;
use serde::{Deserialize, Serialize};
use tracing::debug;

const DEFAULT_OPENAI_ENDPOINT: &str = "https://api.openai.com";

pub struct OpenAiEmbeddings {
    endpoint: String,
    key: String,
    client: reqwest::Client,
}

impl OpenAiEmbeddings {
    pub fn new(api_key: &str) -> Self {
        Self {
            endpoint: DEFAULT_OPENAI_ENDPOINT.to_string(),
            key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn list_embedding_models(&self) -> Vec<(String, usize)> {
        vec![
            (String::from(TEXT_EMBEDDING_3_LARGE), 3072),
            (String::from(TEXT_EMBEDDING_3_SMALL), 1536),
            (String::from(TEXT_EMBEDDING_ADA_002), 1536),
        ]
    }

    pub async fn embed(
        &self,
        content: &[&str],
        model: &str,
    ) -> Result<Vec<Vec<f64>>, EmbeddingError> {
        let request = EmbeddingRequest {
            model: model.to_string(),
            input: content.iter().map(|s| s.to_string()).collect(),
        };

        let response = self
            .client
            .post(format!("{}/v1/embeddings", self.endpoint))
            .bearer_auth(&self.key)
            .json(&request)
            .send()
            .await?
            .json::<EmbeddingResponse>()
            .await?;

        debug!(
            "Embedded {} chunk(s) with '{}', used tokens {}-{} (prompt-total)",
            content.len(),
            response.model,
            response.usage.prompt_tokens,
            response.usage.total_tokens
        );

        Ok(response.data.into_iter().map(|o| o.embedding).collect())
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    object: String,
    data: Vec<EmbeddingObject>,
    model: String,
    usage: Usage,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct EmbeddingObject {
    object: String,
    embedding: Vec<f64>,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: usize,
    total_tokens: usize,
}

const TEXT_EMBEDDING_3_LARGE: &str = "text-embedding-3-large";
const TEXT_EMBEDDING_3_SMALL: &str = "text-embedding-3-small";
const TEXT_EMBEDDING_ADA_002: &str = "text-embedding-ada-002";
