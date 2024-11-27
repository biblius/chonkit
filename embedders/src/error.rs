use thiserror::Error;

use crate::openai::OpenAIError;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("invalid model: {0}")]
    InvalidModel(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[cfg(feature = "fe-local")]
    #[error(transparent)]
    Fastembed(#[from] fastembed::Error),

    #[cfg(any(feature = "openai", feature = "fe-remote"))]
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    /// Contains the error response text in case of OpenAI errors.
    #[cfg(feature = "openai")]
    #[error(transparent)]
    OpenAI(OpenAIError),
}
