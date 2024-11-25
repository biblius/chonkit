use std::{num::ParseIntError, string::FromUtf8Error};
use thiserror::Error;
use tracing::error;
use validify::ValidationErrors;

#[cfg(feature = "qdrant")]
use qdrant_client::QdrantError;

pub mod http;

#[derive(Debug, Error)]
pub enum ChonkitErr {
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

    #[error("embedding error; {0}")]
    Embedding(#[from] chonkit_embedders::error::EmbeddingError),

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
    Chunk(#[from] chunx::ChunkerError),

    #[error("Parse pdf; {0}")]
    ParsePdf(#[from] pdfium_render::prelude::PdfiumError),

    #[error("Docx read; {0}")]
    DocxRead(#[from] docx_rs::ReaderError),

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
}

#[derive(Debug, Error)]
#[error("{error}")]
pub struct ChonkitError {
    file: &'static str,
    line: u32,
    column: u32,
    pub error: ChonkitErr,
}

impl ChonkitError {
    pub fn new(file: &'static str, line: u32, column: u32, error: ChonkitErr) -> ChonkitError {
        ChonkitError {
            file,
            line,
            column,
            error,
        }
    }

    pub fn location(&self) -> String {
        format!("{}:{}:{}", self.file, self.line, self.column)
    }
}

#[macro_export]
macro_rules! err {
    ($ty:ident $(, $l:literal $(,)? $($args:expr),* )?) => {
        Err(ChonkitError::new(
            file!(),
            line!(),
            column!(),
            $crate::error::ChonkitErr::$ty $( (format!($l, $( $args, )*)) )?,
        ))
    };
}

#[macro_export]
macro_rules! map_err {
    ($ex:expr) => {
        $ex.map_err(|e| ChonkitError::new(file!(), line!(), column!(), e.into()))?
    };
}
