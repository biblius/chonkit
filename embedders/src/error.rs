use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("invalid model: {0}")]
    InvalidModel(String),

    #[cfg(feature = "fe-local")]
    #[error(transparent)]
    Fastembed(#[from] fastembed::Error),

    #[cfg(any(feature = "openai", feature = "fe-remote"))]
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}
