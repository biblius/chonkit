use crate::core::model::document::Document;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub(super) struct CreateCollectionPayload {
    pub name: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchPayload {
    pub model: String,
    pub query: String,
    pub collection: String,
    pub limit: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct UploadResult {
    pub documents: Vec<Document>,
    /// Map form keys to errors
    pub errors: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct EmbedPayload {
    pub id: uuid::Uuid,
    pub model: String,
    pub collection: String,
}
