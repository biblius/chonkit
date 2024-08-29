use crate::core::chunk::ChunkerError;
use axum::{http::StatusCode, response::IntoResponse};
use qdrant_client::QdrantError;
use std::{num::ParseIntError, string::FromUtf8Error};
use thiserror::Error;
use tracing::error;

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

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("SQL: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Does not exist: {0}")]
    DoesNotExist(String),

    #[error("Invalid Directory: {0}")]
    InvalidFileName(String),

    #[error("JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Http: {0}")]
    Http(#[from] axum::http::Error),

    #[error("Chunking: {0}")]
    Chunk(#[from] ChunkerError),

    #[error("Unsupport embedding model: {0}")]
    UnsupportedEmbeddingModel(String),

    #[error("Qdrant: {0}")]
    Qdrant(#[from] QdrantError),

    #[error("Parse pdf: {0}")]
    ParsePdf(#[from] lopdf::Error),

    #[error("Docx read: {0}")]
    DocxRead(#[from] docx_rs::ReaderError),

    #[error("Fastembed: {0}")]
    Fastembed(String),
}

impl IntoResponse for ChonkitError {
    fn into_response(self) -> axum::response::Response {
        error!("Error: {self}");

        use ChonkitError as KE;

        match self {
            KE::NotFound(e) => (StatusCode::NOT_FOUND, e).into_response(),
            KE::DoesNotExist(e) => (StatusCode::NOT_FOUND, e).into_response(),
            KE::SerdeJson(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
            }
            // Occurs on pw verification in handlers
            KE::UnsupportedEmbeddingModel(e) => {
                (StatusCode::BAD_REQUEST, e.to_string()).into_response()
            }
            KE::Qdrant(QdrantError::ResponseError { status }) => {
                (StatusCode::BAD_REQUEST, status.to_string()).into_response()
            }
            KE::IO(_)
            | KE::Fastembed(_)
            | KE::Fmt(_)
            | KE::ParseInt(_)
            | KE::Utf8(_)
            | KE::Sqlx(_)
            | KE::Chunk(_)
            | KE::Qdrant(_)
            | KE::InvalidFileName(_)
            | KE::Http(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response(),
            KE::ParsePdf(_) => todo!(),
            KE::DocxRead(_) => todo!(),
        }
    }
}
