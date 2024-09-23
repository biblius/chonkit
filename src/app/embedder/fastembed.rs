use fastembed::{EmbeddingModel, ModelInfo};

#[cfg(all(not(debug_assertions), feature = "fe-local", feature = "fe-remote"))]
compile_error!("only one of 'fe-local' or 'fe-remote' can be enabled when compiling");

#[cfg(feature = "fe-local")]
pub mod local;

#[cfg(feature = "fe-remote")]
pub mod remote;

const DEFAULT_COLLECTION_MODEL: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
const DEFAULT_COLLECTION_SIZE: usize = 384;

pub struct FastEmbedder {
    #[cfg(feature = "fe-local")]
    pub models: std::collections::HashMap<String, fastembed::TextEmbedding>,

    #[cfg(feature = "fe-remote")]
    pub client: reqwest::Client,

    #[cfg(feature = "fe-remote")]
    pub url: String,
}

/// Initialise the FastEmbedder locally.
#[cfg(feature = "fe-local")]
pub fn init() -> FastEmbedder {
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

/// Initialise the FastEmbedder remote client.
#[cfg(feature = "fe-remote")]
pub fn init(url: String) -> FastEmbedder {
    tracing::info!("Initializing remote Fastembed at {url}");
    let client = reqwest::Client::new();
    FastEmbedder { client, url }
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

impl std::fmt::Debug for FastEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastEmbedder").finish()
    }
}

#[cfg(feature = "fe-remote")]
impl FastEmbedder {
    fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.url)
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
