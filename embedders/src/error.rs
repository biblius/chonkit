use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("invalid model: {0}")]
    InvalidModel(String),

    #[cfg(feature = "fe-local")]
    #[error("fastembed error: {0}")]
    Fastembed(#[from] fastembed::Error),

    #[cfg(any(feature = "openai", feature = "fe-remote"))]
    #[error("http client error: {0}")]
    Reqwest(#[from] reqwest::Error),
}
