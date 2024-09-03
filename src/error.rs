use crate::core::chunk::ChunkerError;
use qdrant_client::QdrantError;
use std::{num::ParseIntError, string::FromUtf8Error};
use thiserror::Error;
use tracing::error;
use validify::ValidationErrors;

#[cfg(feature = "http")]
pub mod http;

#[derive(Debug, Error)]
pub enum ChonkitError {
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),

    #[error("FMT: {0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("UTF-8: {0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("Parse int: {0}")]
    ParseInt(#[from] ParseIntError),

    #[error("SQL: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Does not exist: {0}")]
    DoesNotExist(String),

    #[error("Invalid file name: {0}")]
    InvalidFileName(String),

    #[error("File exists: {0}")]
    FileAlreadyExists(String),

    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Chunking: {0}")]
    Chunk(#[from] ChunkerError),

    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),

    #[error("Unsupported embedding model: {0}")]
    UnsupportedEmbeddingModel(String),

    #[error("Qdrant: {0}")]
    Qdrant(#[from] QdrantError),

    #[error("Parse pdf: {0}")]
    ParsePdf(#[from] lopdf::Error),

    #[error("Docx read: {0}")]
    DocxRead(#[from] docx_rs::ReaderError),

    #[error("Fastembed: {0}")]
    Fastembed(String),

    #[error("Validation: {0}")]
    Validation(#[from] ValidationErrors),

    #[cfg(feature = "http")]
    #[error("Http: {0}")]
    Http(#[from] axum::http::Error),
}
