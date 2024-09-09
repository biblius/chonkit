use super::ChonkitError;
use axum::{http::StatusCode, response::IntoResponse, Json};
use qdrant_client::QdrantError;
use serde::Serialize;
use tracing::error;

impl ChonkitError {
    pub fn status(&self) -> StatusCode {
        use ChonkitError as E;
        use StatusCode as SC;
        match self {
            E::IO(_) => todo!(),
            E::Fmt(_) => todo!(),
            E::Utf8(_) => todo!(),
            E::ParseInt(_) | E::AlreadyExists(_) => SC::BAD_REQUEST,
            E::DoesNotExist(_) => SC::NOT_FOUND,
            E::Validation(_)
            | E::Chunk(_)
            | E::InvalidFileName(_)
            | E::UnsupportedFileType(_)
            | E::InvalidEmbeddingModel(_) => SC::UNPROCESSABLE_ENTITY,
            E::ParsePdf(_)
            | E::DocxRead(_)
            | E::Qdrant(_)
            | E::Fastembed(_)
            | E::Sqlx(_)
            | E::Http(_)
            | E::Weaviate(_)
            | E::SerdeJson(_) => SC::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Error response wrapper.
#[cfg(feature = "http")]
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

#[cfg(feature = "http")]
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
            CE::DoesNotExist(e) => (status, ResponseError::new(ET::Api, e)).into_response(),

            CE::SerdeJson(e) => {
                (status, ResponseError::new(ET::Api, e.to_string())).into_response()
            }

            CE::Validation(errors) => (status, ResponseError::new(ET::Api, errors)).into_response(),

            CE::InvalidEmbeddingModel(e) => {
                (status, ResponseError::new(ET::Api, e)).into_response()
            }

            CE::Qdrant(QdrantError::ResponseError { status: st }) => {
                (status, ResponseError::new(ET::Internal, st.to_string())).into_response()
            }

            // TODO
            CE::IO(_)
            | CE::Fastembed(_)
            | CE::UnsupportedFileType(_)
            | CE::Fmt(_)
            | CE::ParseInt(_)
            | CE::Utf8(_)
            | CE::Sqlx(_)
            | CE::Chunk(_)
            | CE::Qdrant(_)
            | CE::InvalidFileName(_)
            | CE::Http(_) => (status, self.to_string()).into_response(),
            CE::ParsePdf(_) => todo!(),
            CE::DocxRead(_) => todo!(),
            CE::AlreadyExists(e) => (status, ResponseError::new(ET::Api, e)).into_response(),
            CE::Weaviate(e) => (status, ResponseError::new(ET::Internal, e)).into_response(),
        }
    }
}
