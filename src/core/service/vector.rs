use crate::core::embedder::Embedder;
use crate::core::repo::document::DocumentRepo;
use crate::core::vector::VectorStore;
use crate::error::ChonkitError;
use crate::DEFAULT_COLLECTION_MODEL;
use std::fmt::Debug;
use tracing::info;

/// # CORE
/// High level operations related to embeddings and vector storage.
#[derive(Debug, Clone)]
pub struct VectorService<R, V, E> {
    repo: R,
    vectors: V,
    embedder: E,
}

impl<R, V, E> VectorService<R, V, E>
where
    R: DocumentRepo,
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

    /// Return a list of all the vector collections.
    pub async fn list_collections(&self) -> Result<Vec<String>, ChonkitError> {
        self.vectors.list_collections().await
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
    }

    /// Create a collection in the vector DB.
    ///
    /// * `name`: Name of the collection.
    /// * `model`: Will be used to determine the collection dimensions.
    pub async fn create_collection(&self, name: &str, model: &str) -> Result<(), ChonkitError> {
        info!("Creating collection '{name}' with embedding model '{model}'",);

        let size = self.embedder.size(model).ok_or_else(|| {
            ChonkitError::UnsupportedEmbeddingModel(format!("Cannot determine size for {model}"))
        })?;

        self.vectors.create_collection(name, size).await?;

        Ok(())
    }

    /// Create and store embeddings in the vector database.
    ///
    /// * `content`: The original chunks.
    /// * `model`: The model to use for embedding.
    /// * `collection`: The collection to store the vectors in.
    pub async fn create_embeddings(
        &self,
        content: Vec<&str>,
        model: &str,
        collection: &str,
    ) -> Result<(), ChonkitError> {
        let embeddings = self.embedder.embed(content.clone(), model).await?;
        self.vectors.store(content, embeddings, collection).await
    }

    /// Query the vector database (semantic search).
    ///
    /// * `model`: The embedding model. The model's embeddings must be the same size as the ones
    /// used in the collection or this will return an error.
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
        let mut embeddings = self.embedder.embed(vec![query], model).await?;
        debug_assert!(!embeddings.is_empty());
        debug_assert_eq!(1, embeddings.len());
        self.vectors
            .query(std::mem::take(&mut embeddings[0]), collection, limit)
            .await
    }
}
