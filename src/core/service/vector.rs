use crate::core::embedder::Embedder;
use crate::core::model::collection::{Collection, CollectionInsert, EmbeddingInsert};
use crate::core::model::{List, Pagination};
use crate::core::repo::vector::VectorRepo;
use crate::core::repo::Atomic;
use crate::core::vector::VectorDb;
use crate::error::ChonkitError;
use crate::{transaction, DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_NAME};
use dto::{CreateCollection, SearchPayload};
use std::fmt::Debug;
use tracing::{error, info};
use uuid::Uuid;
use validify::{Validate, Validify};

/// High level operations related to embeddings (vectors) and their storage.
#[derive(Debug, Clone)]
pub struct VectorService<Repo, V, E> {
    repo: Repo,
    vectors: V,
    embedder: E,
}

impl<R, V, E> VectorService<R, V, E> {
    pub fn new(repo: R, vectors: V, embedder: E) -> Self {
        Self {
            repo,
            vectors,
            embedder,
        }
    }
}

impl<Repo, V, E> VectorService<Repo, V, E>
where
    Repo: VectorRepo<Repo::Tx> + Atomic + Send + Sync,
    Repo::Tx: Send + Sync,
    V: VectorDb + Sync,
    E: Embedder + Sync,
{
    /// List vector collections.
    ///
    /// * `p`: Pagination params.
    pub async fn list_collections(&self, p: Pagination) -> Result<List<Collection>, ChonkitError> {
        p.validate()?;
        self.repo.list_collections(p).await
    }

    /// Get the collection for the given ID.
    ///
    /// * `id`: Collection ID.
    pub async fn get_collection(&self, name: &str) -> Result<Collection, ChonkitError> {
        let collection = self.repo.get_collection(name).await?;
        collection.ok_or_else(|| ChonkitError::DoesNotExist(format!("Collection '{name}'")))
    }

    /// Return a list of models supported by this instance's embedder and their respective sizes.
    pub fn list_embedding_models(&self) -> Vec<(String, usize)> {
        self.embedder.list_embedding_models()
    }

    /// Create the default vector collection if it doesn't already exist.
    pub async fn create_default_collection(&self) {
        self.vectors.create_default_collection().await;

        let insert = CollectionInsert::new(
            DEFAULT_COLLECTION_NAME,
            DEFAULT_COLLECTION_MODEL,
            self.embedder.id(),
            self.vectors.id(),
        );

        match self.repo.insert_collection(insert, None).await {
            Ok(_) => info!("Created default collection '{DEFAULT_COLLECTION_NAME}'"),
            Err(ChonkitError::AlreadyExists(_)) => {
                info!("Default collection '{DEFAULT_COLLECTION_NAME}' already exists")
            }
            Err(e) => error!("Failed to create default collection: {e}"),
        }
    }

    /// Create a collection in the vector DB and store its info in the repository.
    ///
    /// * `data`: Creation data.
    pub async fn create_collection(
        &self,
        mut data: CreateCollection,
    ) -> Result<Collection, ChonkitError> {
        data.validify()?;

        let CreateCollection { name, model } = data.clone();

        let size = self.embedder.size(&model).ok_or_else(|| {
            ChonkitError::InvalidEmbeddingModel(format!(
                "Model {model} not supported by embedder '{}'",
                self.embedder.id()
            ))
        })?;

        info!("Creating collection '{name}' of size '{size}'",);

        let CreateCollection { name, model } = data;

        let mut tx = self.repo.start_tx().await?;

        let collection: Collection = transaction!(Repo, tx, async {
            self.vectors.create_vector_collection(&name, size).await?;

            let insert =
                CollectionInsert::new(&name, &model, self.embedder.id(), self.vectors.id());

            let collection = self.repo.insert_collection(insert, Some(&mut tx)).await?;

            Ok(collection)
        })
        .await?;

        Ok(collection)
    }

    /// Delete a vector collection and all its corresponding embedding entries.
    ///
    /// Returns the amount of embedding entries deleted.
    ///
    /// * `name`: Collection name.
    pub async fn delete_collection(&self, name: &str) -> Result<u64, ChonkitError> {
        self.vectors.delete_vector_collection(name).await?;
        let count = self.repo.delete_all_embeddings(name).await?;
        Ok(count)
    }

    /// Create and store embeddings in the vector database.
    ///
    /// Errors if embeddings already exist in the collection
    /// for the document to prevent duplication in semantic search.
    ///
    /// * `id`: Document ID.
    /// * `collection`: The collection to store the vectors in.
    /// * `chunks`: The chunked document.
    pub async fn create_embeddings(
        &self,
        id: Uuid,
        collection: &str,
        chunks: Vec<&str>,
    ) -> Result<(), ChonkitError> {
        // Make sure the collection exists.
        let Some(collection) = self.repo.get_collection(collection).await? else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection '{collection}'"
            )));
        };

        let existing = self.repo.get_embeddings(id, &collection.name).await?;
        if existing.is_some() {
            return Err(ChonkitError::AlreadyExists(format!(
                "Embeddings for document '{id}' in collection '{}'",
                collection.name
            )));
        }

        let v_collection = self.vectors.get_collection(&collection.name).await?;

        let size = self.embedder.size(&collection.model).ok_or_else(|| {
            ChonkitError::InvalidEmbeddingModel(format!(
                "Model '{}' not supported for embedder {}",
                collection.model,
                self.embedder.id()
            ))
        })?;

        if size != v_collection.size {
            return Err(ChonkitError::InvalidEmbeddingModel(format!(
                "Model size ({size}) not compatible with collection ({})",
                v_collection.size
            )));
        }

        let embeddings = self.embedder.embed(&chunks, &collection.model).await?;

        self.vectors
            .store(&collection.name, &chunks, embeddings)
            .await?;

        let insert = EmbeddingInsert::new(id, &collection.name);

        self.repo.insert_embeddings(insert).await?;

        Ok(())
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

        let collection = self.repo.get_collection(&name).await?;

        let Some(collection) = collection else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection with name {name}"
            )));
        };

        let mut embeddings = self.embedder.embed(&[&query], &collection.model).await?;

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
}

/// Vector service DTOs.
pub mod dto {
    use serde::Deserialize;
    use validify::{field_err, ValidationError, Validify};

    fn ascii_alphanumeric_underscored(s: &str) -> Result<(), ValidationError> {
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(field_err!(
                "ascii_alphanumeric_underscored",
                "must be alphanumeric with underscores [a-z A-Z 0-9 _]"
            ));
        }
        Ok(())
    }

    fn begins_with_ascii_char(s: &str) -> Result<(), ValidationError> {
        if s.starts_with('_') || s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Err(field_err!(
                "begins_with_ascii_char",
                "field must start with a characer [a-zA-Z]"
            ));
        }
        Ok(())
    }

    fn underscore_spaces(s: &mut String) {
        *s = s.replace(' ', "_")
    }

    /// Params for creating collections.
    #[derive(Debug, Clone, Deserialize, Validify)]
    pub struct CreateCollection {
        /// Collection name. Cannot contain special characters.
        #[validate(custom(ascii_alphanumeric_underscored))]
        #[validate(custom(begins_with_ascii_char))]
        #[validate(length(min = 1))]
        #[modify(trim)]
        #[modify(custom(underscore_spaces))]
        pub name: String,

        /// Collection model.
        pub model: String,
    }

    /// Params for semantic search.
    #[derive(Debug, Deserialize, Validify)]
    pub struct SearchPayload {
        /// The text to search by.
        #[modify(trim)]
        pub query: String,

        /// The collection to search in.
        #[validate(length(min = 1))]
        #[modify(trim)]
        pub collection: String,

        /// Amount of results to return.
        pub limit: Option<u32>,
    }
}
