use super::{List, Pagination};
use crate::{
    core::model::document::{Document, DocumentInsert, DocumentUpdate},
    error::ChonkitError,
};
use std::future::Future;

/// Keep tracks of document paths/URLs and additional metadata.
/// Info obtained from here is usually used to load files.
pub trait DocumentRepo {
    /// Get document metadata based on ID.
    fn get_by_id(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<Document>, ChonkitError>> + Send;

    /// Get document metadata by path.
    fn get_by_path(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Option<Document>, ChonkitError>> + Send;

    /// Get a documents's path. A document path can also be a URL,
    /// depending on the storage.
    fn get_path(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<String>, ChonkitError>> + Send;

    /// List documents with limit and offset
    fn list(
        &self,
        p: Pagination,
    ) -> impl Future<Output = Result<List<Document>, ChonkitError>> + Send;

    /// Insert document metadata.
    fn insert(
        &self,
        document: DocumentInsert<'_>,
    ) -> impl Future<Output = Result<Document, ChonkitError>> + Send;

    fn update(
        &self,
        id: uuid::Uuid,
        file: DocumentUpdate<'_>,
    ) -> impl Future<Output = Result<(), ChonkitError>> + Send;

    fn remove_by_id(&self, id: uuid::Uuid)
        -> impl Future<Output = Result<(), ChonkitError>> + Send;

    fn remove_by_path(&self, path: &str) -> impl Future<Output = Result<(), ChonkitError>> + Send;
}
