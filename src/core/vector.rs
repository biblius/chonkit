use uuid::Uuid;

use crate::{core::model::collection::VectorCollection, error::ChonkitError};

/// Vector database operations.
#[async_trait::async_trait]
pub trait VectorDb {
    fn id(&self) -> &'static str;

    /// List available vector collections.
    async fn list_vector_collections(&self) -> Result<Vec<VectorCollection>, ChonkitError>;

    /// Create a vector collection.
    ///
    /// * `name`: The name of the collection.
    /// * `size`: Vector size of the collection.
    async fn create_vector_collection(&self, name: &str, size: usize) -> Result<(), ChonkitError>;

    /// Get collection info.
    ///
    /// * `name`: Collection name.
    async fn get_collection(&self, name: &str) -> Result<VectorCollection, ChonkitError>;

    /// Delete a vector collection.
    ///
    /// * `name`: The name of the collection.
    async fn delete_vector_collection(&self, name: &str) -> Result<(), ChonkitError>;

    /// Used to create the initial collection.
    ///
    /// This is part of the interface in order to handle errors more efficiently,
    /// such as the collection already existing.
    ///
    /// As this method is called only on app start, it's fine to panic if something
    /// goes wrong.
    ///
    /// * `size`: The vector size of the collection.
    async fn create_default_collection(&self, size: usize);

    /// Perform semantic search.
    ///
    /// * `search`: The query to use as the search vector.
    /// * `collection`: The collection to search in.
    /// * `limit`: Amount of results to return.
    async fn query(
        &self,
        search: Vec<f32>,
        collection: &str,
        limit: u32,
    ) -> Result<Vec<String>, ChonkitError>;

    /// Store the contents and their vectors to the vector storage.
    /// The `contents` and `vectors` inputs are expected to
    /// be 1:1, i.e. the same index into both lists should
    /// yield the contents and their respectful embeddings.
    ///
    /// * `content`: The contents to append to the vectors.
    /// * `vectors`: The vectors to store.
    /// * `collection`: The vector collection to store in.
    async fn insert_embeddings(
        &self,
        document_id: Uuid,
        collection: &str,
        content: &[&str],
        vectors: Vec<Vec<f32>>,
    ) -> Result<(), ChonkitError>;

    /// Delete the vectors tagged with the given `document_id`.
    ///
    /// * `collection`: The collection to delete from.
    /// * `document_id`: The id of the document whose vectors to delete.
    async fn delete_embeddings(
        &self,
        collection: &str,
        document_id: Uuid,
    ) -> Result<(), ChonkitError>;

    /// Returns the amount of vectors tagged with the given `document_id`.
    ///
    /// * `collection`: The collection to count in.
    /// * `document_id`: The id of the document whose vectors to count.
    async fn count_vectors(
        &self,
        collection: &str,
        document_id: Uuid,
    ) -> Result<usize, ChonkitError>;
}
