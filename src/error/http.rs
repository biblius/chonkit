use super::ChonkitError;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use tracing::error;

impl ChonkitError {
    pub fn status(&self) -> StatusCode {
        use ChonkitError as E;
        use StatusCode as SC;
        match self {
            E::ParseInt(_) => SC::BAD_REQUEST,
            E::AlreadyExists(_) => SC::CONFLICT,
            E::DoesNotExist(_) => SC::NOT_FOUND,
            E::Validation(_)
            | E::Regex(_)
            | E::Chunk(_)
            | E::InvalidFileName(_)
            | E::UnsupportedFileType(_)
            | E::InvalidProvider(_)
            | E::InvalidEmbeddingModel(_) => SC::UNPROCESSABLE_ENTITY,
            E::ParsePdf(_)
            | E::DocxRead(_)
            | E::Fastembed(_)
            | E::Sqlx(_)
            | E::Http(_)
            | E::IO(_)
            | E::Fmt(_)
            | E::Utf8(_)
            | E::Batch
            | E::SerdeJson(_) => SC::INTERNAL_SERVER_ERROR,

            #[cfg(any(feature = "openai", feature = "fe-remote"))]
            E::Reqwest(e) => e.status().unwrap_or(SC::INTERNAL_SERVER_ERROR),

            #[cfg(feature = "qdrant")]
            E::Qdrant(_) => SC::INTERNAL_SERVER_ERROR,

            #[cfg(feature = "weaviate")]
            E::Weaviate(_) => SC::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Error response wrapper.
#[derive(Debug, Serialize)]
struct ResponseError<T: Serialize> {
    error_type: ErrorType,
    body: T,
}

impl<T> ResponseError<T>
where
    T: Serialize,
{
    pub fn new(error_type: ErrorType, body: T) -> Self {
        Self { error_type, body }
    }
}

#[derive(Debug, Serialize)]
enum ErrorType {
    Internal,
    Api,
}

impl<T> IntoResponse for ResponseError<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        <Json<ResponseError<T>> as IntoResponse>::into_response(Json(self))
    }
}

#[cfg(feature = "http")]
impl IntoResponse for ChonkitError {
    fn into_response(self) -> axum::response::Response {
        error!("{self}");

        let status = self.status();

        use ChonkitError as CE;
        use ErrorType as ET;

        match self {
            CE::InvalidProvider(e) => (status, ResponseError::new(ET::Api, e)).into_response(),
            CE::DoesNotExist(e) => (status, ResponseError::new(ET::Api, e)).into_response(),

            CE::SerdeJson(e) => {
                (status, ResponseError::new(ET::Api, e.to_string())).into_response()
            }

            CE::Validation(errors) => (status, ResponseError::new(ET::Api, errors)).into_response(),

            CE::InvalidEmbeddingModel(e) => {
                (status, ResponseError::new(ET::Api, e)).into_response()
            }

            CE::Batch => {
                (status, ResponseError::new(ET::Internal, self.to_string())).into_response()
            }

            // TODO
            CE::IO(_)
            | CE::Regex(_)
            | CE::Fastembed(_)
            | CE::UnsupportedFileType(_)
            | CE::Fmt(_)
            | CE::ParseInt(_)
            | CE::Utf8(_)
            | CE::Sqlx(_)
            | CE::Chunk(_)
            | CE::InvalidFileName(_)
            | CE::Http(_) => (status, self.to_string()).into_response(),
            CE::ParsePdf(_) => todo!(),
            CE::DocxRead(_) => todo!(),
            CE::AlreadyExists(e) => (status, ResponseError::new(ET::Api, e)).into_response(),

            #[cfg(any(feature = "openai", feature = "fe-remote"))]
            CE::Reqwest(e) => {
                (status, ResponseError::new(ET::Internal, e.to_string())).into_response()
            }

            #[cfg(feature = "weaviate")]
            CE::Weaviate(e) => (status, ResponseError::new(ET::Internal, e)).into_response(),

            #[cfg(feature = "qdrant")]
            CE::Qdrant(qdrant_client::QdrantError::ResponseError { status: st }) => {
                (status, ResponseError::new(ET::Internal, st.to_string())).into_response()
            }

            #[cfg(feature = "qdrant")]
            CE::Qdrant(_) => (status, self.to_string()).into_response(),
        }
    }
}
