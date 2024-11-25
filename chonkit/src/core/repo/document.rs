use crate::{
    core::{
        chunk::ChunkConfig,
        document::parser::ParseConfig,
        model::{
            document::{
                config::{DocumentChunkConfig, DocumentParseConfig},
                Document, DocumentConfig, DocumentDisplay, DocumentInsert, DocumentUpdate,
            },
            List, PaginationSort,
        },
    },
    error::ChonkitError,
};
use uuid::Uuid;

use super::Atomic;

/// Keep tracks of documents and their chunking/parsing configurations.
/// Info obtained from here is usually used to load files.
#[async_trait::async_trait]
pub trait DocumentRepo {
    /// Get document metadata based on ID.
    ///
    /// * `id`: Document ID.
    async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<Document>, ChonkitError>;

    /// Get full document configuration based on ID (including chunker and parser).
    ///
    /// * `id`: Document ID.
    async fn get_config_by_id(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<DocumentConfig>, ChonkitError>;

    /// Get document metadata by path.
    ///
    /// * `path`: Document path.
    async fn get_by_path(&self, path: &str) -> Result<Option<Document>, ChonkitError>;

    /// Get a documents's path. A document path can also be a URL,
    /// depending on the storage.
    ///
    /// * `id`: Document ID.
    async fn get_path(&self, id: uuid::Uuid) -> Result<Option<String>, ChonkitError>;

    /// Get a document by its content hash.
    ///
    /// * `hash`: Document content hash.
    async fn get_by_hash(&self, hash: &str) -> Result<Option<Document>, ChonkitError>;

    /// List documents with limit and offset
    ///
    /// * `p`: Pagination params.
    async fn list(
        &self,
        p: PaginationSort,
        src: Option<&str>,
        ready: Option<bool>,
    ) -> Result<List<Document>, ChonkitError>;

    /// List documents with limit and offset with additional relations for embeddings.
    ///
    /// * `p`: Pagination params.
    /// * `src`: Optional source to filter by.
    /// * `document_id`: Optional document ID to filter by.
    async fn list_with_collections(
        &self,
        p: PaginationSort,
        src: Option<&str>,
        document_id: Option<Uuid>,
    ) -> Result<List<DocumentDisplay>, ChonkitError>;

    /// Insert document metadata.
    ///
    /// * `document`: Insert payload.
    async fn insert(&self, document: DocumentInsert<'_>) -> Result<Document, ChonkitError>;

    /// Update document metadata.
    ///
    /// * `id`: Document ID.
    /// * `document`: Update payload.
    async fn update(
        &self,
        id: uuid::Uuid,
        document: DocumentUpdate<'_>,
    ) -> Result<u64, ChonkitError>;

    /// Remove document metadata by id.
    ///
    /// * `id`: Document ID.
    async fn remove_by_id(&self, id: uuid::Uuid) -> Result<u64, ChonkitError>;

    /// Remove document metadata by path.
    ///
    /// * `path`: Document path.
    async fn remove_by_path(&self, path: &str) -> Result<u64, ChonkitError>;

    /// Get the document's configuration for chunking.
    ///
    /// * `id`: Document ID.
    async fn get_chunk_config(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<DocumentChunkConfig>, ChonkitError>;

    /// Get the document's configuration for parsing.
    ///
    ///
    /// * `id`: Document ID.
    async fn get_parse_config(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<DocumentParseConfig>, ChonkitError>;

    /// Insert or update the document's configuration for chunking.
    ///
    /// * `document_id`: Document ID.
    /// * `chunker`: Chunking configuration.
    async fn upsert_chunk_config(
        &self,
        document_id: uuid::Uuid,
        chunker: ChunkConfig,
    ) -> Result<DocumentChunkConfig, ChonkitError>;

    /// Insert or update the document's configuration for parsing.
    ///
    /// * `document_id`: Document ID.
    /// * `config`: Parsing configuration.
    async fn upsert_parse_config(
        &self,
        document_id: uuid::Uuid,
        config: ParseConfig,
    ) -> Result<DocumentParseConfig, ChonkitError>;

    /// Insert document metadata and the configurations for parsing and chunking in a transaction.
    ///
    /// * `document`: Document insert payload.
    /// * `parse_config`: Parsing configuration.
    /// * `chunk_config`: Chunking configuration.
    /// * `tx`: The transaction to run in.
    async fn insert_with_configs(
        &self,
        document: DocumentInsert<'_>,
        parse_config: ParseConfig,
        chunk_config: ChunkConfig,
        tx: &mut <Self as Atomic>::Tx,
    ) -> Result<DocumentConfig, ChonkitError>
    where
        Self: Atomic;
}
