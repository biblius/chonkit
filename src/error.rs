use crate::core::chunk::ChunkerError;
use std::{num::ParseIntError, string::FromUtf8Error};
use thiserror::Error;
use tracing::error;
use validify::ValidationErrors;

#[cfg(feature = "qdrant")]
use qdrant_client::QdrantError;

pub mod http;

#[derive(Debug, Error)]
pub enum ChonkitError {
    #[error("Unable to send job to batch executor")]
    Batch,

    #[error("Does not exist; {0}")]
    DoesNotExist(String),

    #[error("Invalid file name; {0}")]
    InvalidFileName(String),

    #[error("Entity already exists; {0}")]
    AlreadyExists(String),

    #[error("Unsupported file type; {0}")]
    UnsupportedFileType(String),

    #[error("Invalid embedding model; {0}")]
    InvalidEmbeddingModel(String),

    #[error("Invalid provider; {0}")]
    InvalidProvider(String),

    #[error("IO; {0}")]
    IO(#[from] std::io::Error),

    #[error("FMT; {0}")]
    Fmt(#[from] std::fmt::Error),

    #[error("UTF-8; {0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("Parse int; {0}")]
    ParseInt(#[from] ParseIntError),

    #[error("SQL; {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("JSON error; {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Chunking; {0}")]
    Chunk(#[from] ChunkerError),

    #[error("Parse pdf; {0}")]
    ParsePdf(#[from] pdfium_render::prelude::PdfiumError),

    #[error("Docx read; {0}")]
    DocxRead(#[from] docx_rs::ReaderError),

    #[error("Fastembed; {0}")]
    Fastembed(String),

    #[error("Validation; {0}")]
    Validation(#[from] ValidationErrors),

    #[error("Regex; {0}")]
    Regex(#[from] regex::Error),

    #[error("Http; {0}")]
    Http(#[from] axum::http::Error),

    #[cfg(feature = "qdrant")]
    #[error("Qdrant; {0}")]
    Qdrant(#[from] QdrantError),

    #[cfg(feature = "weaviate")]
    #[error("Weaviate; {0}")]
    Weaviate(String),

    #[error("Axum; {0}")]
    Axum(#[from] axum::Error),

    #[cfg(any(feature = "openai", feature = "fe-remote"))]
    #[error("Openai; {0}")]
    Reqwest(#[from] reqwest::Error),
}
