use crate::error::EmbeddingError;
use fastembed::{EmbeddingModel, ModelInfo};
use ort::execution_providers::CPUExecutionProvider;
#[cfg(feature = "cuda")]
use ort::execution_providers::CUDAExecutionProvider;

pub struct LocalFastEmbedder {
    pub models: std::collections::HashMap<String, fastembed::TextEmbedding>,
}

impl LocalFastEmbedder {
    /// Initialise the FastEmbedder locally.
    pub fn new() -> Self {
        tracing::info!("Initializing local Fastembed");
        #[cfg(feature = "cuda")]
        {
            use ort::execution_providers::ExecutionProvider;
            tracing::info!(
                "Using CUDA: {:?}",
                ExecutionProvider::is_available(&CUDAExecutionProvider::default())
            );
        }

        let mut models = std::collections::HashMap::new();

        for model in list_models() {
            tracing::info!("Setting up text embedding model: {}", model.model_code);
            let embedding = fastembed::TextEmbedding::try_new(
                fastembed::InitOptions::new(model.model)
                    .with_execution_providers(vec![
                        #[cfg(feature = "cuda")]
                        CUDAExecutionProvider::default().into(),
                        CPUExecutionProvider::default().into(),
                    ])
                    .with_show_download_progress(true),
            )
            .unwrap_or_else(|e| panic!("error while instantiating text embedding model: {e}"));

            models.insert(model.model_code.to_string(), embedding);
        }
        Self { models }
    }

    /// Initialize the FastEmbedder with a specific model.
    /// Useful for tests. If `model` is `None`, the default model will be used.
    #[doc(hidden)]
    pub fn new_with_model(model: &str) -> Self {
        let mut models = std::collections::HashMap::new();

        for m in fastembed::TextEmbedding::list_supported_models() {
            if m.model_code != model {
                continue;
            }

            tracing::info!("Setting up text embedding model: {}", m.model_code);

            let embedding = fastembed::TextEmbedding::try_new(
                fastembed::InitOptions::new(m.model)
                    .with_execution_providers(vec![
                        #[cfg(feature = "cuda")]
                        CUDAExecutionProvider::default().into(),
                        CPUExecutionProvider::default().into(),
                    ])
                    .with_show_download_progress(true),
            )
            .unwrap_or_else(|e| panic!("error while instantiating text embedding model: {e}"));

            models.insert(m.model_code.to_string(), embedding);
        }

        LocalFastEmbedder { models }
    }

    pub fn list_models(&self) -> Vec<ModelInfo<EmbeddingModel>> {
        list_models()
    }

    pub fn embed(&self, content: &[&str], model: &str) -> Result<Vec<Vec<f64>>, EmbeddingError> {
        let embedder = self.models.get(model).ok_or_else(|| {
            EmbeddingError::InvalidModel(format!("model '{model}' not supported by fastembed",))
        })?;

        let embeddings = embedder.embed(content.to_vec(), None).unwrap();

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

impl Default for LocalFastEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for LocalFastEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastEmbedder").finish()
    }
}
