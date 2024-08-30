use super::{List, Pagination};
use crate::{
    core::model::document::{
        config::{DocumentConfig, DocumentConfigInsert},
        Document, DocumentInsert, DocumentUpdate,
    },
    error::ChonkitError,
};
use std::future::Future;

/// Keep tracks of document.
/// Info obtained from here is usually used to load files.
pub trait DocumentRepo {
    /// Get document metadata based on ID.
    ///
    /// * `id`: Document ID.
    fn get_by_id(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<Document>, ChonkitError>> + Send;

    /// Get document metadata by path.
    ///
    /// * `path`: Document path.
    fn get_by_path(
        &self,
        path: &str,
    ) -> impl Future<Output = Result<Option<Document>, ChonkitError>> + Send;

    /// Get a documents's path. A document path can also be a URL,
    /// depending on the storage.
    ///
    /// * `id`: Document ID.
    fn get_path(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<String>, ChonkitError>> + Send;

    /// List documents with limit and offset
    ///
    /// * `p`: Pagination params.
    fn list(
        &self,
        p: Pagination,
    ) -> impl Future<Output = Result<List<Document>, ChonkitError>> + Send;

    /// Insert document metadata.
    ///
    /// * `document`: Insert payload.
    fn insert(
        &self,
        document: DocumentInsert<'_>,
    ) -> impl Future<Output = Result<Document, ChonkitError>> + Send;

    /// Update document metadata.
    ///
    /// * `id`: Document ID.
    /// * `document`: Update payload.
    fn update(
        &self,
        id: uuid::Uuid,
        document: DocumentUpdate<'_>,
    ) -> impl Future<Output = Result<(), ChonkitError>> + Send;

    /// Remove document metadata by id.
    ///
    /// * `id`: Document ID.
    fn remove_by_id(&self, id: uuid::Uuid)
        -> impl Future<Output = Result<(), ChonkitError>> + Send;

    /// Remove document metadata by path.
    ///
    /// * `path`: Document path.
    fn remove_by_path(&self, path: &str) -> impl Future<Output = Result<(), ChonkitError>> + Send;

    /// Get the document's configuration for chunking/parsing.
    ///
    /// * `id`: Document ID.
    fn get_config(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<DocumentConfig>, ChonkitError>> + Send;

    fn insert_config(
        &self,
        config: DocumentConfigInsert,
    ) -> impl Future<Output = Result<DocumentConfig, ChonkitError>> + Send;
}
