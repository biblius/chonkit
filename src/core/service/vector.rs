use crate::core::embedder::Embedder;
use crate::core::model::collection::{Collection, CollectionInsert};
use crate::core::model::{List, Pagination};
use crate::core::repo::vector::VectorRepo;
use crate::core::vector::store::VectorStore;
use crate::error::ChonkitError;
use crate::DEFAULT_COLLECTION_MODEL;
use std::fmt::Debug;
use tracing::info;
use validify::Validify;

use super::document::dto::CreateCollectionPayload;

pub mod dto;

/// High level operations related to embeddings and vector storage.
#[derive(Debug, Clone)]
pub struct VectorService<R, V, E> {
    repo: R,
    vectors: V,
    embedder: E,
}

impl<R, V, E> VectorService<R, V, E>
where
    R: VectorRepo,
    V: VectorStore,
    E: Embedder,
{
    pub fn new(repo: R, vectors: V, embedder: E) -> Self {
        Self {
            repo,
            vectors,
            embedder,
        }
    }

    /// Return a list of models supported by this instance's embedder.
    pub fn list_embedding_models(&self) -> Vec<String> {
        self.embedder.list_embedding_models()
    }

    /// Create the default vector collection if it doesn't already exist.
    pub async fn create_default_collection(&self) {
        let size = self
            .embedder
            .size(DEFAULT_COLLECTION_MODEL)
            .expect("invalid default model");

        self.vectors.create_default_collection(size).await;

        // Default collection will always have a nil UUID
        let collection = CollectionInsert::default();

        self.repo
            .insert_collection(collection)
            .await
            .expect("error while inserting default collection");
    }

    pub async fn get_collection(&self, id: uuid::Uuid) -> Result<Collection, ChonkitError> {
        let collection = self.repo.get_collection(id).await?;
        collection.ok_or_else(|| ChonkitError::DoesNotExist(format!("Collection with ID {id}")))
    }

    /// Create a collection in the vector DB and store its info in the repository.
    ///
    /// * `name`: Name of the collection.
    /// * `model`: Will be used to determine the collection dimensions.
    pub async fn create_collection(
        &self,
        mut payload: CreateCollectionPayload,
    ) -> Result<Collection, ChonkitError> {
        payload.validify()?;

        let CreateCollectionPayload { name, model } = payload;

        info!("Creating collection '{name}' with embedding model '{model}'",);

        let size = self.embedder.size(&model).ok_or_else(|| {
            ChonkitError::UnsupportedEmbeddingModel(format!("Cannot determine size for {model}"))
        })?;

        self.vectors.create_collection(&name, size).await?;

        let collection = CollectionInsert::new(&name, &model);
        let collection = self.repo.insert_collection(collection).await?;

        Ok(collection)
    }

    /// List vector collections.
    ///
    /// * `p`: Pagination params.
    pub async fn list_collections(&self, p: Pagination) -> Result<List<Collection>, ChonkitError> {
        self.repo.list(p).await
    }

    /// Delete a vector collection from the repository and the store.
    ///
    /// * `id`: Collection ID.
    pub async fn delete_collection(&self, id: uuid::Uuid) -> Result<(), ChonkitError> {
        let collection = self.repo.get_collection(id).await?;

        let Some(collection) = collection else {
            return Ok(());
        };

        self.vectors.delete_collection(&collection.name).await?;
        self.repo.delete_collection(collection.id).await?;

        Ok(())
    }

    /// Create and store embeddings in the vector database.
    ///
    /// * `content`: The original chunks.
    /// * `model`: The model to use for embedding.
    /// * `collection`: The collection to store the vectors in.
    pub async fn create_embeddings(
        &self,
        id: uuid::Uuid,
        content: Vec<String>,
        collection: &Collection,
    ) -> Result<(), ChonkitError> {
        let embeddings = self
            .embedder
            .embed(content.clone(), &collection.model)
            .await?;
        self.vectors
            .store(content, embeddings, &collection.name)
            .await
    }

    /// Query the vector database (semantic search).
    ///
    /// * `model`: The embedding model. The model's embeddings must be the same size as the ones
    ///    used in the collection or this will return an error.
    /// * `query`: The text to search by.
    /// * `collection`: The collection to search in.
    /// * `limit`: Amount of results returned.
    pub async fn search(
        &self,
        model: &str,
        query: &str,
        collection: &str,
        limit: u64,
    ) -> Result<Vec<String>, ChonkitError> {
        let mut embeddings = self.embedder.embed(vec![query.to_string()], model).await?;
        debug_assert!(!embeddings.is_empty());
        debug_assert_eq!(1, embeddings.len());
        self.vectors
            .query(std::mem::take(&mut embeddings[0]), collection, limit)
            .await
    }
}
