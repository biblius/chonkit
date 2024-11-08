//! Http specific DTOs.

use crate::core::{
    chunk::Chunker,
    document::parser::ParseConfig,
    model::{document::DocumentConfig, Pagination, PaginationSort},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validify::{schema_err, schema_validation, Validate, ValidationErrors};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct UploadResult {
    pub documents: Vec<DocumentConfig>,
    /// Map form keys to errors
    pub errors: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct ConfigUpdatePayload {
    /// Parsing configuration.
    pub parser: Option<ParseConfig>,

    /// Chunking configuration.
    pub chunker: Option<Chunker>,
}

/// Used for single embeddings.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct EmbeddingSinglePayload {
    /// The ID of the document to embed.
    pub document: Uuid,

    /// The ID of the collection in which to store the embeddings to.
    pub collection: Uuid,
}

/// Used for batch embeddings.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
#[validate(Self::validate_schema)]
pub(super) struct EmbeddingBatchPayload {
    /// The documents to embed and add to the collection.
    pub add: Vec<Uuid>,

    /// The documents to remove from the collection.
    pub remove: Vec<Uuid>,

    /// The ID of the collection in which to store the embeddings to.
    pub collection: Uuid,
}

impl EmbeddingBatchPayload {
    #[schema_validation]
    fn validate_schema(&self) -> Result<(), ValidationErrors> {
        if self.add.is_empty() && self.remove.is_empty() {
            schema_err! {
                "no_documents",
                "either `add` or `remove` must contain document IDs"
            }
        }
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListEmbeddingsPayload {
    /// Limit and offset
    #[validate]
    #[serde(flatten)]
    #[param(inline)]
    pub pagination: Pagination,

    /// Filter by collection.
    pub collection: Option<Uuid>,
}

#[derive(Debug, Default, Deserialize, Validate, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListDocumentsPayload {
    /// Limit and offset
    #[validate]
    #[serde(flatten)]
    #[param(inline)]
    pub pagination: PaginationSort,

    /// Filter by file source.
    pub src: Option<String>,

    /// Filter by document ID.
    pub document_id: Option<Uuid>,
}
