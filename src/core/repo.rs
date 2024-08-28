use serde::{Deserialize, Serialize};

use crate::{
    error::ChonkitError,
    model::document::{Document, DocumentInsert, DocumentUpdate},
};

#[derive(Debug, Serialize)]
pub struct List<T> {
    pub total: Option<usize>,
    pub items: Vec<T>,
}

impl<T> List<T> {
    pub fn new(total: Option<usize>, items: Vec<T>) -> Self {
        Self { total, items }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Pagination {
    pub page: usize,
    pub per_page: usize,
}

/// Keep tracks of document paths/URLs and additional metadata.
/// Info obtained from here is usually used to load files.
pub trait DocumentRepo {
    /// Get document metadata based on ID.
    async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<Document>, ChonkitError>;

    /// Get document metadata by path.
    async fn get_by_path(&self, path: &str) -> Result<Option<Document>, ChonkitError>;

    /// Get a documents's path. A document path can also be a URL,
    /// depending on the storage.
    async fn get_path(&self, id: uuid::Uuid) -> Result<Option<String>, ChonkitError>;

    /// List documents with limit and offset
    async fn list(&self, p: Pagination) -> Result<List<Document>, ChonkitError>;

    /// Insert document metadata.
    async fn insert(&self, document: DocumentInsert<'_>) -> Result<Document, ChonkitError>;

    async fn update(&self, id: uuid::Uuid, file: DocumentUpdate<'_>) -> Result<(), ChonkitError>;

    async fn remove_by_id(&self, id: uuid::Uuid) -> Result<(), ChonkitError>;

    async fn remove_by_path(&self, path: &str) -> Result<(), ChonkitError>;
}
