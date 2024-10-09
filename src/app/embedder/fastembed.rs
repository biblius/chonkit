#[cfg(all(not(debug_assertions), feature = "fe-local", feature = "fe-remote"))]
compile_error!("only one of 'fe-local' or 'fe-remote' can be enabled when compiling");

#[cfg(feature = "fe-local")]
pub use local::FastEmbedder;

#[cfg(feature = "fe-remote")]
pub use remote::FastEmbedder;

const DEFAULT_COLLECTION_MODEL: &str = "Xenova/bge-base-en-v1.5";
const DEFAULT_COLLECTION_SIZE: usize = 768;

/// Embedder implementation for fastembed when running it
/// locally.
#[cfg(feature = "fe-local")]
pub mod local {
    use super::{DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_SIZE};
    use crate::{core::embedder::Embedder, error::ChonkitError};
    use fastembed::{EmbeddingModel, ModelInfo};

    pub struct FastEmbedder {
        pub models: std::collections::HashMap<String, fastembed::TextEmbedding>,
    }

    impl FastEmbedder {
        /// Initialise the FastEmbedder locally.
        pub fn new() -> FastEmbedder {
            tracing::info!("Initializing local Fastembed");
            let mut models = std::collections::HashMap::new();

            for model in list_models() {
                tracing::info!("Setting up text embedding model: {}", model.model_code);
                let embedding = fastembed::TextEmbedding::try_new(
                    fastembed::InitOptions::new(model.model)
                        .with_execution_providers(vec![
                            #[cfg(feature = "cuda")]
                            ort::CUDAExecutionProvider::default().into(),
                            ort::CPUExecutionProvider::default().into(),
                        ])
                        .with_show_download_progress(true),
                )
                .unwrap_or_else(|e| panic!("error while instantiating text embedding model: {e}"));

                models.insert(model.model_code.to_string(), embedding);
            }

            FastEmbedder { models }
        }
    }

    fn list_models() -> Vec<ModelInfo<EmbeddingModel>> {
        const MODEL_LIST: &[EmbeddingModel] = &[
            EmbeddingModel::BGESmallENV15,
            EmbeddingModel::BGELargeENV15,
            EmbeddingModel::BGEBaseENV15,
            EmbeddingModel::AllMiniLML6V2,
            EmbeddingModel::AllMiniLML12V2,
        ];

        fastembed::TextEmbedding::list_supported_models()
            .into_iter()
            .filter(|model| MODEL_LIST.contains(&model.model))
            .collect()
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
            Ok(list_models()
                .into_iter()
                .map(|model| (model.model_code, model.dim))
                .collect())
        }

        async fn embed(
            &self,
            content: &[&str],
            model: &str,
        ) -> Result<Vec<Vec<f32>>, ChonkitError> {
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

    impl std::fmt::Debug for FastEmbedder {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FastEmbedder").finish()
        }
    }
}

/// Embedder implementation for running fastembed on a remote
/// machine supporting CUDA.
#[cfg(feature = "fe-remote")]
pub mod remote {
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
            let response: HashMap<String, usize> =
                self.client.get(&url).send().await?.json().await?;

            Ok(response
                .into_iter()
                .map(|(model, size)| (model, size))
                .collect())
        }

        async fn embed(
            &self,
            content: &[&str],
            model: &str,
        ) -> Result<Vec<Vec<f32>>, ChonkitError> {
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

    impl std::fmt::Debug for FastEmbedder {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FastEmbedder").finish()
        }
    }
}

/// Initialize the FastEmbedder with a specific model.
/// Useful for tests. If `model` is `None`, the default model will be used.
#[cfg(all(test, feature = "fe-local", not(feature = "fe-remote")))]
pub fn init_single(model: Option<&str>) -> FastEmbedder {
    let mut models = std::collections::HashMap::new();

    let model = model.unwrap_or(DEFAULT_COLLECTION_MODEL);

    for m in fastembed::TextEmbedding::list_supported_models() {
        if m.model_code != model {
            continue;
        }

        tracing::info!("Setting up text embedding model: {}", m.model_code);

        let embedding = fastembed::TextEmbedding::try_new(
            fastembed::InitOptions::new(m.model)
                .with_execution_providers(vec![
                    #[cfg(feature = "cuda")]
                    ort::CUDAExecutionProvider::default().into(),
                    ort::CPUExecutionProvider::default().into(),
                ])
                .with_show_download_progress(true),
        )
        .unwrap_or_else(|e| panic!("error while instantiating text embedding model: {e}"));

        models.insert(m.model_code.to_string(), embedding);
    }

    FastEmbedder { models }
}
