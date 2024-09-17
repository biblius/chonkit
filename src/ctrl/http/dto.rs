//! Http specific DTOs.

use crate::core::{model::document::Document, service::vector::dto::CreateCollection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;
use validify::{schema_err, schema_validation, ValidationErrors, Validify};

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
#[validate(Self::validate)]
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
    fn validate(&self) -> Result<(), ValidationErrors> {
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
