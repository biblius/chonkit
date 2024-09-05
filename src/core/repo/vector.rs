use crate::{
    core::model::collection::{Collection, CollectionInsert},
    core::model::{List, Pagination},
    error::ChonkitError,
};
use std::future::Future;
use uuid::Uuid;

/// Keeps track of vector collections and vector related metadata.
pub trait VectorRepo {
    /// Insert collection metadata.
    ///
    /// * `name`: Collection name.
    /// * `model`: Collection embedding model.
    fn insert_collection(
        &self,
        collection: CollectionInsert<'_>,
    ) -> impl Future<Output = Result<Collection, ChonkitError>> + Send;

    /// Get collection metadata.
    ///
    /// * `id`: Collection ID.
    fn get_collection(
        &self,
        id: Uuid,
    ) -> impl Future<Output = Result<Option<Collection>, ChonkitError>> + Send;

    /// Get collection metadata by name.
    /// Collections have unique names.
    ///
    /// * `name`: Collection name.
    fn get_collection_by_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<Collection>, ChonkitError>> + Send;

    /// Delete collection metadata.
    ///
    /// * `id`: Collection ID.
    fn delete_collection(&self, id: Uuid)
        -> impl Future<Output = Result<u64, ChonkitError>> + Send;

    /// List collections with limit and offset
    ///
    /// * `p`: Pagination params.
    fn list(
        &self,
        p: Pagination,
    ) -> impl Future<Output = Result<List<Collection>, ChonkitError>> + Send;

    /// Update the default model of a collection.
    /// Callers must ensure the new model's embedding
    /// size is the same as the existing.
    ///
    /// * `id`: Collection ID.
    /// * `model`: The new default model for the collection.
    fn update_model(
        &self,
        id: Uuid,
        model: &str,
    ) -> impl Future<Output = Result<(), ChonkitError>> + Send;
}
