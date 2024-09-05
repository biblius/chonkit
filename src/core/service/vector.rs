use crate::core::embedder::Embedder;
use crate::core::model::collection::{Collection, CollectionInsert};
use crate::core::model::{List, Pagination};
use crate::core::repo::vector::VectorRepo;
use crate::core::vector::store::VectorStore;
use crate::error::ChonkitError;
use dto::{CreateCollectionPayload, SearchPayload};
use std::fmt::Debug;
use tracing::{debug, info};
use uuid::Uuid;
use validify::Validify;

/// High level operations related to embeddings and vector storage.
#[derive(Debug, Clone)]
pub struct VectorService<R, V, E> {
    repo: R,
    vectors: V,
    embedder: E,
}

impl<R, V, E> VectorService<R, V, E>
where
    R: VectorRepo + Sync,
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

    /// List vector collections.
    ///
    /// * `p`: Pagination params.
    pub async fn list_collections(&self, p: Pagination) -> Result<List<Collection>, ChonkitError> {
        self.repo.list(p).await
    }

    /// Return a list of models supported by this instance's embedder and their respective sizes.
    pub fn list_embedding_models(&self) -> Vec<(String, usize)> {
        self.embedder.list_embedding_models()
    }

    /// Create the default vector collection if it doesn't already exist.
    pub async fn create_default_collection(&self) {
        self.vectors.create_default_collection().await;

        // Default collection will always have a nil UUID
        let collection = CollectionInsert::default();

        self.repo
            .insert_collection(collection)
            .await
            .expect("error while inserting default collection");
    }

    /// Get the collection for the given ID.
    ///
    /// * `id`: Collection ID.
    pub async fn get_collection(&self, id: Uuid) -> Result<Collection, ChonkitError> {
        let collection = self.repo.get_collection(id).await?;
        collection.ok_or_else(|| ChonkitError::DoesNotExist(format!("Collection with ID {id}")))
    }

    /// Create a collection in the vector DB and store its info in the repository.
    ///
    /// * `payload`: Parser and chunking configuration.
    pub async fn create_collection(
        &self,
        mut payload: CreateCollectionPayload,
    ) -> Result<Collection, ChonkitError> {
        payload.validify()?;

        let CreateCollectionPayload { name, model } = payload;

        info!("Creating collection '{name}' with embedding model '{model}'",);

        let size = self.embedder.size(&model).ok_or_else(|| {
            ChonkitError::InvalidEmbeddingModel(format!("Cannot determine size for {model}"))
        })?;

        self.vectors.create_collection(&name, size).await?;

        let collection = CollectionInsert::new(&name, size as usize).with_model(&model);
        let collection = self.repo.insert_collection(collection).await?;

        Ok(collection)
    }

    /// Delete a vector collection from the repository and the store.
    ///
    /// * `id`: Collection ID.
    pub async fn delete_collection(&self, id: Uuid) -> Result<(), ChonkitError> {
        let collection = self.repo.get_collection(id).await?;

        let Some(collection) = collection else {
            return Ok(());
        };

        self.vectors.delete_collection(&collection.name).await?;
        self.repo.delete_collection(collection.id).await?;

        Ok(())
    }

    /// Update the default model of a collection.
    /// The model's embedding size must be the same as the existing one's.
    ///
    /// * `id`: Collection ID.
    /// * `model`: New default model to use.
    pub async fn update_default_model(&self, id: Uuid, model: &str) -> Result<(), ChonkitError> {
        let collection = self.repo.get_collection(id).await?;

        let Some(collection) = collection else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection with id {id}"
            )));
        };

        let new_size = self.embedder.size(model).ok_or_else(|| {
            ChonkitError::InvalidEmbeddingModel(format!("Cannot determine size for {model}"))
        })?;

        if collection.size != new_size as usize {
            return Err(ChonkitError::InvalidEmbeddingModel(format!(
                "Embedding size mismatch, got {new_size} - required {}",
                collection.size
            )));
        }

        self.repo.update_model(id, model).await
    }

    /// Create and store embeddings in the vector database.
    ///
    /// * `id`: Document ID.
    /// * `content`: The original chunks.
    /// * `collection`: The collection to store the vectors in.
    pub async fn create_embeddings(
        &self,
        id: Uuid,
        content: Vec<String>,
        collection: &Collection,
    ) -> Result<(), ChonkitError> {
        let model = if let Some(model) = collection.model.clone() {
            model
        } else {
            self.find_compatible_model(collection.id, collection.size)?
        };

        let embeddings = self.embedder.embed(content.clone(), &model).await?;

        self.vectors
            .store(content, embeddings, &collection.name)
            .await
    }

    /// Query the vector database (semantic search).
    /// Limit defaults to 5.
    ///
    /// * `input`: Search params.
    pub async fn search(&self, mut input: SearchPayload) -> Result<Vec<String>, ChonkitError> {
        input.validify()?;

        let SearchPayload {
            query,
            collection: name,
            limit,
        } = input;

        let collection = self.repo.get_collection_by_name(&name).await?;

        let Some(collection) = collection else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection with name {name}"
            )));
        };

        let model = if let Some(model) = collection.model.clone() {
            model
        } else {
            self.find_compatible_model(collection.id, collection.size)?
        };

        let mut embeddings = self.embedder.embed(vec![query.to_string()], &model).await?;

        debug_assert!(!embeddings.is_empty());
        debug_assert_eq!(1, embeddings.len());

        self.vectors
            .query(
                std::mem::take(&mut embeddings[0]),
                &collection.name,
                limit.unwrap_or(5),
            )
            .await
    }

    pub async fn sync(&self) -> Result<(), ChonkitError> {
        self.vectors.sync(&self.repo).await
    }

    fn find_compatible_model(&self, id: Uuid, size: usize) -> Result<String, ChonkitError> {
        debug!("Collection {id} does not have a default model specified, searching for compatible ones.");
        let model = self
            .list_embedding_models()
            .into_iter()
            .find(|(_, s)| *s == size)
            .map(|(m, _)| m)
            .ok_or_else(|| {
                ChonkitError::InvalidEmbeddingModel(format!(
                    "No model found for embedding size {size}",
                ))
            })?;
        debug!("Defaulted to {model} for collection {id}");
        Ok(model)
    }
}

/// Vector service DTOs.
pub mod dto {
    use serde::Deserialize;
    use validify::Validify;

    /// Params for creating collections.
    #[derive(Debug, Deserialize, Validify)]
    pub struct CreateCollectionPayload {
        /// Collection name.
        #[validate(length(min = 1))]
        #[modify(trim)]
        pub name: String,

        /// Default collection model.
        #[validate(length(min = 1))]
        #[modify(trim)]
        pub model: String,
    }

    /// Params for creating embeddings.
    #[derive(Debug, Deserialize)]
    pub struct EmbedPayload {
        pub document_id: uuid::Uuid,
        pub collection_id: uuid::Uuid,
    }

    /// Params for semantic search.
    #[derive(Debug, Deserialize, Validify)]
    pub struct SearchPayload {
        /// The text to search by.
        pub query: String,

        /// The collection to search in.
        #[validate(length(min = 1))]
        #[modify(trim)]
        pub collection: String,

        /// Amount of results to return.
        pub limit: Option<u64>,
    }
}
