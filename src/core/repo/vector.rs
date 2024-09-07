use crate::{
    core::model::{
        collection::{Collection, CollectionInsert, Embedding, EmbeddingInsert},
        List, Pagination,
    },
    error::ChonkitError,
};
use std::future::Future;
use uuid::Uuid;

/// Keeps track of vector collections and vector related metadata.
pub trait VectorRepo {
    /// List collections with limit and offset
    ///
    /// * `p`: Pagination params.
    fn list_collections(
        &self,
        p: Pagination,
    ) -> impl Future<Output = Result<List<Collection>, ChonkitError>> + Send;

    /// Insert collection metadata.
    ///
    /// * `name`: Collection name.
    /// * `model`: Collection embedding model.
    fn upsert_collection(
        &self,
        insert: CollectionInsert<'_>,
    ) -> impl Future<Output = Result<Collection, ChonkitError>> + Send;

    /// Delete a vector collection.
    ///
    /// * `name`: The name of the collection.
    fn delete_collection(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<u64, ChonkitError>> + Send;

    /// Get collection metadata.
    ///
    /// * `id`: Collection ID.
    fn get_collection(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<Collection>, ChonkitError>> + Send;

    /// Insert embedding metadata.
    ///
    /// * `embeddings`: Insert payload.
    fn insert_embeddings(
        &self,
        embeddings: EmbeddingInsert<'_>,
    ) -> impl Future<Output = Result<Embedding, ChonkitError>> + Send;

    /// Get a document's embedding information.
    ///
    /// * `id`: Document ID.
    fn get_all_embeddings(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<Vec<Embedding>, ChonkitError>> + Send;

    /// Get a document's embedding information for the given collection.
    ///
    /// * `id`: Document ID.
    fn get_embeddings(
        &self,
        id: Uuid,
        collection: &str,
    ) -> impl Future<Output = Result<Option<Embedding>, ChonkitError>> + Send;

    /// Delete embedding info for a document in the given collection.
    /// Return the amount of entries deleted.
    ///
    /// * `id`: Document ID.
    /// * `collection`: Collection name.
    fn delete_embeddings(
        &self,
        id: Uuid,
        collection: &str,
    ) -> impl Future<Output = Result<u64, ChonkitError>> + Send;

    /// Delete all embedding entries for the given collection.
    ///
    /// * `collection`: Collection name.
    fn delete_all_embeddings(
        &self,
        collection: &str,
    ) -> impl Future<Output = Result<u64, ChonkitError>> + Send;
}
