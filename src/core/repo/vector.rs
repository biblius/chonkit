use crate::{
    core::model::collection::{Collection, CollectionInsert},
    core::model::{List, Pagination},
    error::ChonkitError,
};
use std::future::Future;

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
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<Option<Collection>, ChonkitError>> + Send;

    /// Delete collection metadata.
    ///
    /// * `id`: Collection ID.
    fn delete_collection(
        &self,
        id: uuid::Uuid,
    ) -> impl Future<Output = Result<u64, ChonkitError>> + Send;

    /// List collections with limit and offset
    ///
    /// * `p`: Pagination params.
    fn list(
        &self,
        p: Pagination,
    ) -> impl Future<Output = Result<List<Collection>, ChonkitError>> + Send;
}
