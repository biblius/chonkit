use super::{DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_SIZE};
use crate::{core::embedder::Embedder, error::ChonkitError};
use fastembed::{EmbeddingModel, ModelInfo};

pub struct FastEmbedder {
    pub models: std::collections::HashMap<String, fastembed::TextEmbedding>,
}

impl FastEmbedder {
    /// Initialise the FastEmbedder locally.
    pub fn new() -> Self {
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
        Self { models }
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

    async fn embed(&self, content: &[&str], model: &str) -> Result<Vec<Vec<f64>>, ChonkitError> {
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

        Ok(embeddings
            .into_iter()
            .map(|e| e.into_iter().map(|e| e as f64).collect())
            .collect())
    }
}

impl std::fmt::Debug for FastEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastEmbedder").finish()
    }
}

/// Initialize the FastEmbedder with a specific model.
/// Useful for tests. If `model` is `None`, the default model will be used.
#[cfg(test)]
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
