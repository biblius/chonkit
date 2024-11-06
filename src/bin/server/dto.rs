//! Http specific DTOs.

use chonkit::core::{
    chunk::Chunker,
    document::parser::ParseConfig,
    model::{document::Document, Pagination},
    service::vector::dto::CreateCollection,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validify::{schema_err, schema_validation, Validate, ValidationErrors, Validify};

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct UploadResult {
    pub documents: Vec<Document>,
    /// Map form keys to errors
    pub errors: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct CreateCollectionPayload {
    /// Collection name. Cannot contain special characters.
    pub name: String,

    /// Collection model.
    pub model: String,

    /// Vector database provider.
    pub vector_provider: String,

    /// Embeddings provider.
    pub embedding_provider: String,
}

impl From<CreateCollectionPayload> for CreateCollection {
    fn from(value: CreateCollectionPayload) -> Self {
        CreateCollection {
            name: value.name,
            model: value.model,
        }
    }
}

/// Params for semantic search.
#[derive(Debug, Deserialize, Validify, ToSchema)]
#[serde(rename_all = "camelCase")]
#[validate(Self::validate_schema)]
pub(super) struct SearchPayload {
    /// The text to search by.
    #[modify(trim)]
    pub query: String,

    /// The collection to search in. Has priority over
    /// everything else.
    pub collection_id: Option<Uuid>,

    /// If given search via the name and provider combo.
    #[validate(length(min = 1))]
    #[modify(trim)]
    pub collection_name: Option<String>,

    /// Vector provider.
    pub provider: Option<String>,

    /// Amount of results to return.
    pub limit: Option<u32>,
}

impl SearchPayload {
    #[schema_validation]
    fn validate_schema(&self) -> Result<(), ValidationErrors> {
        let SearchPayload {
            collection_id,
            collection_name,
            provider,
            ..
        } = self;
        match (collection_id, collection_name, provider) {
            (None, None, None) => {
                schema_err!(
                        "either_id_or_name_and_provider",
                        "one of either `collection_id`, or `provider` and `collection_name` combination must be set"
                    );
            }
            (None, Some(_), None) | (None, None, Some(_)) => {
                schema_err!(
                    "name_and_provider",
                    "both 'collection_name'and 'provider' must be set if `collection_id` is not set"
                );
            }
            _ => {}
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct ConfigUpdatePayload {
    /// Parsing configuration.
    pub parser: Option<ParseConfig>,

    /// Chunking configuration.
    pub chunker: Option<Chunker>,
}

/// DTO used for previewing chunks.
#[derive(Debug, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
#[validate(Self::validate_schema)]
pub(super) struct ChunkPreviewPayload {
    /// Parsing configuration.
    pub parser: Option<ParseConfig>,

    /// Chunking configuration.
    pub chunker: Chunker,

    /// The embedding provider to use. Necessary
    /// when using the semantic chunker.
    pub embedder: Option<String>,
}

impl ChunkPreviewPayload {
    #[schema_validation]
    fn validate_schema(&self) -> Result<(), ValidationErrors> {
        if let (Chunker::Semantic(_), None) = (&self.chunker, &self.embedder) {
            schema_err! {
                "chunker_params",
                "`embedder` must be set when using semantic chunker"
            };
        }
    }
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
    pub pagination: Pagination,

    /// Filter by file source.
    pub src: Option<String>,

    /// Filter by document ID.
    pub document_id: Option<Uuid>,
}
