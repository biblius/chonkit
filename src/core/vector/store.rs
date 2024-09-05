use crate::{core::repo::vector::VectorRepo, error::ChonkitError};
use std::future::Future;

/// Vector database operations.
pub trait VectorStore {
    /// List available vector collections.
    fn list_collections(&self) -> impl Future<Output = Result<Vec<String>, ChonkitError>> + Send;

    /// Create a vector collection.
    ///
    /// * `name`: The name of the collection.
    /// * `size`: Vector size of the collection.
    fn create_collection(
        &self,
        name: &str,
        size: u64,
    ) -> impl Future<Output = Result<(), ChonkitError>> + Send;

    /// Delete a vector collection.
    ///
    /// * `name`: The name of the collection.
    fn delete_collection(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<(), ChonkitError>> + Send;

    /// Used to create the initial collection.
    ///
    /// This is part of the interface in order to handle errors more efficiently,
    /// such as the collection already existing.
    ///
    /// As this method is called only on app start, it's fine to panic if something
    /// goes wrong.
    fn create_default_collection(&self) -> impl Future<Output = ()> + Send;

    /// Perform semantic search.
    ///
    /// * `search`: The query to use as the search vector.
    /// * `collection`: The collection to search in.
    /// * `limit`: Amount of results to return.
    fn query(
        &self,
        search: Vec<f32>,
        collection: &str,
        limit: u64,
    ) -> impl Future<Output = Result<Vec<String>, ChonkitError>> + Send;

    /// Store the contents and their vectors to the vector storage.
    /// The `contents` and `vectors` inputs are expected to
    /// be 1:1, i.e. the same index into both lists should
    /// yield the contents and their respectful embeddings.
    ///
    /// * `content`: The contents to append to the vectors.
    /// * `vectors`: The vectors to store.
    /// * `collection`: The vector collection to store in.
    fn store(
        &self,
        content: Vec<String>,
        vectors: Vec<Vec<f32>>,
        collection: &str,
    ) -> impl Future<Output = Result<(), ChonkitError>>;

    /// Sync repository contents with the store.
    ///
    /// * `repo`: Vector collection repository.
    fn sync(
        &self,
        repo: &(impl VectorRepo + Sync),
    ) -> impl Future<Output = Result<(), ChonkitError>> + Send;
}
