use crate::core::embedder::Embedder;
use crate::core::model::collection::{
    Collection, CollectionDisplay, CollectionInsert, Embedding, EmbeddingInsert,
};
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
    Repo: VectorRepo + Atomic + Send + Sync,
    Repo::Tx: Send + Sync,
{
    /// List vector collections.
    ///
    /// * `p`: Pagination params.
    pub async fn list_collections(&self, p: Pagination) -> Result<List<Collection>, ChonkitError> {
        p.validate()?;
        self.repo.list_collections(p).await
    }

    pub async fn list_collections_display(
        &self,
        p: Pagination,
    ) -> Result<List<CollectionDisplay>, ChonkitError> {
        p.validate()?;
        self.repo.list_collections_display(p).await
    }

    /// Get the collection for the given ID.
    ///
    /// * `id`: Collection ID.
    pub async fn get_collection(&self, id: Uuid) -> Result<Collection, ChonkitError> {
        let collection = self.repo.get_collection(id).await?;
        collection.ok_or_else(|| ChonkitError::DoesNotExist(format!("Collection with ID '{id}'")))
    }

    /// Get the collection for the given ID with additional info for display purposes.
    ///
    /// * `id`: Collection ID.
    pub async fn get_collection_display(
        &self,
        id: Uuid,
    ) -> Result<CollectionDisplay, ChonkitError> {
        let collection = self.repo.get_collection_display(id).await?;
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
    ///
    /// * `embedder`: The embedder to use.
    pub async fn list_embedding_models(
        &self,
        embedder: &(dyn Embedder + Send + Sync),
    ) -> Result<Vec<(String, usize)>, ChonkitError> {
        Ok(embedder.list_embedding_models().await?)
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

        let size = embedder.size(&model).await?.ok_or_else(|| {
            ChonkitError::InvalidEmbeddingModel(format!(
                "Model {model} not supported by embedder '{}'",
                embedder.id()
            ))
        })?;

        info!("Creating collection '{name}' of size '{size}'",);

        let CreateCollection { name, model } = data;

        let collection: Collection = transaction!(self.repo, |tx| async move {
            vector_db.create_vector_collection(&name, size).await?;

            let insert = CollectionInsert::new(&name, &model, embedder.id(), vector_db.id());

            let collection = self.repo.insert_collection(insert, Some(tx)).await?;

            Ok(collection)
        })?;

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
    /// * `vector_db`: The vector DB implementation to use.
    /// * `embedder`: The embedder to use.
    pub async fn create_embeddings(
        &self,
        vector_db: &(dyn VectorDb + Send + Sync),
        embedder: &(dyn Embedder + Send + Sync),
        CreateEmbeddings {
            id,
            collection,
            chunks,
        }: CreateEmbeddings<'_>,
    ) -> Result<Embedding, ChonkitError> {
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

        let size = embedder.size(&collection.model).await?.ok_or_else(|| {
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

        debug_assert_eq!(chunks.len(), embeddings.len());

        vector_db
            .insert_embeddings(id, &collection.name, &chunks, embeddings)
            .await?;

        let embeddings = self
            .repo
            .insert_embeddings(EmbeddingInsert::new(id, collection.id))
            .await?;

        Ok(embeddings)
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

    pub async fn get_embeddings(
        &self,
        document_id: Uuid,
        collection_id: Uuid,
    ) -> Result<Option<Embedding>, ChonkitError> {
        self.repo.get_embeddings(document_id, collection_id).await
    }

    pub async fn list_embeddings(
        &self,
        pagination: Pagination,
        collection_id: Option<Uuid>,
    ) -> Result<List<Embedding>, ChonkitError> {
        self.repo.list_embeddings(pagination, collection_id).await
    }

    pub async fn delete_embeddings(
        &self,
        collection_id: Uuid,
        document_id: Uuid,
        vector_db: &(dyn VectorDb + Send + Sync),
    ) -> Result<u64, ChonkitError> {
        let Some(collection) = self.repo.get_collection(collection_id).await? else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection with ID '{collection_id}'"
            )));
        };

        vector_db
            .delete_embeddings(&collection.name, document_id)
            .await?;

        let amount_deleted = self
            .repo
            .delete_embeddings(document_id, collection_id)
            .await?;

        Ok(amount_deleted)
    }

    pub async fn count_embeddings(
        &self,
        collection_id: Uuid,
        document_id: Uuid,
        vector_db: &(dyn VectorDb + Send + Sync),
    ) -> Result<usize, ChonkitError> {
        let Some(collection) = self.repo.get_collection(collection_id).await? else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection with ID '{collection_id}'"
            )));
        };
        vector_db.count_vectors(&collection.name, document_id).await
    }
}

/// Vector service DTOs.
pub mod dto {
    use crate::core::model::collection::Collection;
    use uuid::Uuid;
    use validify::{field_err, ValidationError, Validify};

    fn ascii_alphanumeric_underscored(s: &str) -> Result<(), ValidationError> {
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(field_err!(
                "collection_name",
                "collection name must be alphanumeric with underscores [a-z A-Z 0-9 _]"
            ));
        }
        Ok(())
    }

    fn begins_with_capital_ascii_letter(s: &str) -> Result<(), ValidationError> {
        if s.starts_with('_')
            || s.chars()
                .next()
                .is_some_and(|c| !c.is_ascii_alphabetic() || c.is_lowercase())
        {
            return Err(field_err!(
                "collection_name",
                "collection name must start with a capital characer [A-Z]"
            ));
        }
        Ok(())
    }

    /// Params for creating collections.
    #[derive(Debug, Clone, Validify)]
    pub struct CreateCollection {
        /// Collection name. Cannot contain special characters.
        #[validate(custom(ascii_alphanumeric_underscored))]
        #[validate(custom(begins_with_capital_ascii_letter))]
        #[validate(length(min = 1))]
        #[modify(trim)]
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
        pub chunks: &'a [&'a str],
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
