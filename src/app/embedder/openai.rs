use http::{HeaderMap, HeaderValue};
use reqwest::header;
use serde::{Deserialize, Serialize};

use crate::core::embedder::Embedder;
use crate::error::ChonkitError;

pub struct OpenAiEmbeddings {
    endpoint: String,
    client: reqwest::Client,
}

impl OpenAiEmbeddings {
    pub fn new(endpoint: &str, api_key: &str) -> Self {
        let mut headers = HeaderMap::new();

        let mut auth = HeaderValue::from_str(&format!("Bearer: {api_key}")).unwrap();
        auth.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth);

        Self {
            endpoint: endpoint.to_string(),
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .expect("unable to build client"),
        }
    }
}

#[async_trait::async_trait]
impl Embedder for OpenAiEmbeddings {
    fn id(&self) -> &'static str {
        "openai"
    }

    fn default_model(&self) -> (String, usize) {
        (String::from(TEXT_EMBEDDING_ADA_002), 1536)
    }

    fn list_embedding_models(&self) -> Vec<(String, usize)> {
        vec![
            (String::from(TEXT_EMBEDDING_3_LARGE), 1536),
            (String::from(TEXT_EMBEDDING_3_SMALL), 3072),
            (String::from(TEXT_EMBEDDING_ADA_002), 1536),
        ]
    }

    async fn embed(&self, content: &[&str], model: &str) -> Result<Vec<Vec<f32>>, ChonkitError> {
        let request = EmbeddingRequest {
            model: model.to_string(),
            input: content.iter().map(|s| s.to_string()).collect(),
        };

        let response = self
            .client
            .post(self.endpoint.as_str())
            .json(&request)
            .send()
            .await?
            .json::<EmbeddingResponse>()
            .await?;

        Ok(response.data.into_iter().map(|o| o.embedding).collect())
    }

    fn size(&self, model: &str) -> Option<usize> {
        self.list_embedding_models()
            .into_iter()
            .find(|m| m.0 == model)
            .map(|m| m.1)
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    object: String,
    data: Vec<EmbeddingObject>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingObject {
    object: String,
    embedding: Vec<f32>,
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
