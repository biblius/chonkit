use crate::core::embedder::Embedder;
use crate::core::model::collection::{Collection, CollectionInsert, EmbeddingInsert};
use crate::core::model::{List, Pagination};
use crate::core::repo::vector::VectorRepo;
use crate::core::repo::Atomic;
use crate::core::vector::VectorDb;
use crate::error::ChonkitError;
use crate::{transaction, DEFAULT_COLLECTION_NAME};
use dto::{CreateCollection, CreateEmbeddings, Search};
use tracing::{error, info};
use uuid::Uuid;
use validify::{Validate, Validify};

/// High level operations related to embeddings (vectors) and their storage.
#[derive(Clone)]
pub struct VectorService<Repo> {
    repo: Repo,
}

impl<R> VectorService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<Repo> VectorService<Repo>
where
    Repo: VectorRepo<Repo::Tx> + Atomic + Send + Sync,
    Repo::Tx: Send + Sync,
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
    pub async fn get_collection(&self, id: Uuid) -> Result<Collection, ChonkitError> {
        let collection = self.repo.get_collection(id).await?;
        collection.ok_or_else(|| ChonkitError::DoesNotExist(format!("Collection with ID '{id}'")))
    }

    /// Get the collection for the given name and provider unique combo.
    ///
    /// * `name`: Collection name.
    /// * `provider`: Vector provider.
    pub async fn get_collection_by_name(
        &self,
        name: &str,
        provider: &str,
    ) -> Result<Collection, ChonkitError> {
        let collection = self.repo.get_collection_by_name(name, provider).await?;
        collection.ok_or_else(|| ChonkitError::DoesNotExist(format!("Collection '{name}'")))
    }

    /// Return a list of models supported by this instance's embedder and their respective sizes.
    pub fn list_embedding_models(&self, embedder: &dyn Embedder) -> Vec<(String, usize)> {
        embedder.list_embedding_models()
    }

    /// Create the default vector collection if it doesn't already exist.
    pub async fn create_default_collection(
        &self,
        vector_db: &(dyn VectorDb + Send + Sync),
        embedder: &(dyn Embedder + Send + Sync),
    ) {
        let (model, size) = embedder.default_model();

        vector_db.create_default_collection(size).await;

        let insert = CollectionInsert::new(
            DEFAULT_COLLECTION_NAME,
            &model,
            embedder.id(),
            vector_db.id(),
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
        vector_db: &(dyn VectorDb + Send + Sync),
        embedder: &(dyn Embedder + Send + Sync),
        mut data: CreateCollection,
    ) -> Result<Collection, ChonkitError> {
        data.validify()?;

        let CreateCollection { name, model } = data.clone();

        let size = embedder.size(&model).ok_or_else(|| {
            ChonkitError::InvalidEmbeddingModel(format!(
                "Model {model} not supported by embedder '{}'",
                embedder.id()
            ))
        })?;

        info!("Creating collection '{name}' of size '{size}'",);

        let CreateCollection { name, model } = data;

        let mut tx = self.repo.start_tx().await?;

        let collection: Collection = transaction!(Repo, tx, async {
            vector_db.create_vector_collection(&name, size).await?;

            let insert = CollectionInsert::new(&name, &model, embedder.id(), vector_db.id());

            let collection = self.repo.insert_collection(insert, Some(&mut tx)).await?;

            Ok(collection)
        })
        .await?;

        Ok(collection)
    }

    /// Delete a vector collection and all its corresponding embedding entries.
    /// It is assumed the vector provider has a collection with the name
    /// equal to the one found in the collection with the given ID.
    ///
    /// * `id`: Collection ID.
    pub async fn delete_collection(
        &self,
        vector_db: &(dyn VectorDb + Send + Sync),
        id: Uuid,
    ) -> Result<u64, ChonkitError> {
        let Some(collection) = self.repo.get_collection(id).await? else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection with ID '{id}'"
            )));
        };
        vector_db.delete_vector_collection(&collection.name).await?;
        let count = self.repo.delete_collection(id).await?;
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
        vector_db: &(dyn VectorDb + Send + Sync),
        embedder: &(dyn Embedder + Send + Sync),
        CreateEmbeddings {
            id,
            collection,
            chunks,
        }: CreateEmbeddings<'_>,
    ) -> Result<(), ChonkitError> {
        // Make sure the collection exists.
        let Some(collection) = self.repo.get_collection(collection).await? else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection '{collection}'"
            )));
        };

        let existing = self.repo.get_embeddings(id, collection.id).await?;
        if existing.is_some() {
            return Err(ChonkitError::AlreadyExists(format!(
                "Embeddings for document '{id}' in collection '{}'",
                collection.name
            )));
        }

        let v_collection = vector_db.get_collection(&collection.name).await?;

        let size = embedder.size(&collection.model).ok_or_else(|| {
            ChonkitError::InvalidEmbeddingModel(format!(
                "Model '{}' not supported for embedder {}",
                collection.model,
                embedder.id()
            ))
        })?;

        if size != v_collection.size {
            return Err(ChonkitError::InvalidEmbeddingModel(format!(
                "Model size ({size}) not compatible with collection ({})",
                v_collection.size
            )));
        }

        let embeddings = embedder.embed(&chunks, &collection.model).await?;

        vector_db
            .store(&collection.name, &chunks, embeddings)
            .await?;

        let insert = EmbeddingInsert::new(id, collection.id);

        self.repo.insert_embeddings(insert).await?;

        Ok(())
    }

    /// Query the vector database (semantic search).
    /// Limit defaults to 5.
    ///
    /// * `input`: Search params.
    pub async fn search(
        &self,
        vector_db: &(dyn VectorDb + Send + Sync),
        embedder: &(dyn Embedder + Send + Sync),
        input: Search,
    ) -> Result<Vec<String>, ChonkitError> {
        let Search {
            query,
            limit,
            collection,
        } = input;

        let mut embeddings = embedder.embed(&[&query], &collection.model).await?;

        debug_assert!(!embeddings.is_empty());
        debug_assert_eq!(1, embeddings.len());

        vector_db
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
    use uuid::Uuid;
    use validify::{field_err, ValidationError, Validify};

    use crate::core::model::collection::Collection;

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
    #[derive(Debug, Clone, Validify)]
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

    #[derive(Debug, Clone, Validify)]
    pub struct CreateEmbeddings<'a> {
        /// Document ID.
        pub id: Uuid,

        /// Which collection these embeddings are for.
        pub collection: Uuid,

        /// The chunked document.
        pub chunks: Vec<&'a str>,
    }

    /// Params for semantic search.
    #[derive(Debug)]
    pub struct Search {
        /// The collection to search in.
        pub collection: Collection,

        /// The text to search by.
        pub query: String,

        /// Amount of results to return.
        pub limit: Option<u32>,
    }
}
