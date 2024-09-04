use crate::{
    core::{
        chunk::Chunker,
        document::parser::ParseConfig,
        model::{
            document::{
                config::{DocumentChunkConfig, DocumentParseConfig},
                Document, DocumentConfig, DocumentInsert, DocumentUpdate,
            },
            List, Pagination,
        },
    },
    error::ChonkitError,
};
use std::future::Future;

/// Keep tracks of documents and their chunking/parsing configurations.
/// Info obtained from here is usually used to load files.
pub trait DocumentRepo {
    /// Get document metadata based on ID.
    ///
    /// * `id`: Document ID.
    fn get_by_id(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<Document>, ChonkitError>> + Send;

    /// Get full document configuration based on ID (including chunker and parser).
    ///
    /// * `id`: Document ID.
    fn get_config_by_id(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<DocumentConfig>, ChonkitError>> + Send;

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

    /// Get a document by its content hash.
    ///
    /// * `hash`: Document content hash.
    fn get_by_hash(
        &self,
        hash: &str,
    ) -> impl Future<Output = Result<Option<Document>, ChonkitError>> + Send;

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
    ) -> impl Future<Output = Result<u64, ChonkitError>> + Send;

    /// Remove document metadata by id.
    ///
    /// * `id`: Document ID.
    fn remove_by_id(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<u64, ChonkitError>> + Send;

    /// Remove document metadata by path.
    ///
    /// * `path`: Document path.
    fn remove_by_path(&self, path: &str) -> impl Future<Output = Result<u64, ChonkitError>> + Send;

    /// Get the document's configuration for chunking.
    ///
    /// * `id`: Document ID.
    fn get_chunk_config(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<DocumentChunkConfig>, ChonkitError>> + Send;

    /// Get the document's configuration for parsing.
    ///
    ///
    /// * `id`: Document ID.
    fn get_parse_config(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<DocumentParseConfig>, ChonkitError>> + Send;

    fn upsert_chunk_config(
        &self,
        document_id: uuid::Uuid,
        chunker: Chunker,
    ) -> impl Future<Output = Result<DocumentChunkConfig, ChonkitError>> + Send;

    fn upsert_parse_config(
        &self,
        document_id: uuid::Uuid,
        config: ParseConfig,
    ) -> impl Future<Output = Result<DocumentParseConfig, ChonkitError>> + Send;
}
