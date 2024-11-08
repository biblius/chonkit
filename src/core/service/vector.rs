use crate::core::model::collection::{
    Collection, CollectionDisplay, CollectionInsert, Embedding, EmbeddingInsert,
};
use crate::core::model::{List, Pagination, PaginationSort};
use crate::core::provider::ProviderState;
use crate::core::repo::vector::VectorRepo;
use crate::core::repo::Atomic;
use crate::error::ChonkitError;
use crate::{transaction, DEFAULT_COLLECTION_NAME};
use dto::{CreateCollectionPayload, CreateEmbeddings, SearchPayload};
use tracing::{error, info};
use uuid::Uuid;
use validify::{Validate, Validify};

/// High level operations related to embeddings (vectors) and their storage.
#[derive(Clone)]
pub struct VectorService<Repo> {
    repo: Repo,
    providers: ProviderState,
}

impl<R> VectorService<R> {
    pub fn new(repo: R, providers: ProviderState) -> Self {
        Self { repo, providers }
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
    pub async fn list_collections(
        &self,
        p: PaginationSort,
    ) -> Result<List<Collection>, ChonkitError> {
        p.validate()?;
        self.repo.list_collections(p).await
    }

    pub async fn list_collections_display(
        &self,
        p: PaginationSort,
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
        embedder: &str,
    ) -> Result<Vec<(String, usize)>, ChonkitError> {
        let embedder = self.providers.embedding.provider(embedder)?;
        Ok(embedder.list_embedding_models().await?)
    }

    /// Create the default vector collection if it doesn't already exist.
    pub async fn create_default_collection(&self, vector_db: &str, embedder: &str) {
        let vector_db = self
            .providers
            .vector
            .provider(vector_db)
            .expect("invalid vector provider");
        let embedder = self
            .providers
            .embedding
            .provider(embedder)
            .expect("invalid embedding provider");

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
        mut data: CreateCollectionPayload,
    ) -> Result<Collection, ChonkitError> {
        data.validify()?;

        let CreateCollectionPayload {
            name,
            model,
            vector_provider,
            embedding_provider,
        } = data;

        let vector_db = self.providers.vector.provider(&vector_provider)?;
        let embedder = self.providers.embedding.provider(&embedding_provider)?;

        let size = embedder.size(&model).await?.ok_or_else(|| {
            ChonkitError::InvalidEmbeddingModel(format!(
                "Model {model} not supported by embedder '{}'",
                embedder.id()
            ))
        })?;

        info!("Creating collection '{name}' of size '{size}'",);

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
    pub async fn delete_collection(&self, id: Uuid) -> Result<u64, ChonkitError> {
        let Some(collection) = self.repo.get_collection(id).await? else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection with ID '{id}'"
            )));
        };
        let vector_db = self.providers.vector.provider(&collection.provider)?;
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

        let vector_db = self.providers.vector.provider(&collection.provider)?;
        let embedder = self.providers.embedding.provider(&collection.embedder)?;

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
    pub async fn search(&self, mut search: SearchPayload) -> Result<Vec<String>, ChonkitError> {
        search.validify()?;

        let collection = if let Some(collection_id) = search.collection_id {
            self.get_collection(collection_id).await?
        } else {
            let (Some(name), Some(provider)) = (&search.collection_name, &search.provider) else {
                // Cannot happen because of above validify
                return Err(ChonkitError::InvalidProvider(
                format!("Both 'collection_name' and 'provider' must be provided if 'collection_id' is not provided"),
            ));
            };

            self.get_collection_by_name(name, provider).await?
        };

        let vector_db = self.providers.vector.provider(&collection.provider)?;
        let embedder = self.providers.embedding.provider(&collection.embedder)?;

        let mut embeddings = embedder.embed(&[&search.query], &collection.model).await?;

        debug_assert!(!embeddings.is_empty());
        debug_assert_eq!(1, embeddings.len());

        vector_db
            .query(
                std::mem::take(&mut embeddings[0]),
                &collection.name,
                search.limit.unwrap_or(5),
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
        pagination.validate()?;
        self.repo.list_embeddings(pagination, collection_id).await
    }

    pub async fn delete_embeddings(
        &self,
        collection_id: Uuid,
        document_id: Uuid,
    ) -> Result<u64, ChonkitError> {
        let Some(collection) = self.repo.get_collection(collection_id).await? else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection with ID '{collection_id}'"
            )));
        };

        let vector_db = self.providers.vector.provider(&collection.provider)?;

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
    ) -> Result<usize, ChonkitError> {
        let Some(collection) = self.repo.get_collection(collection_id).await? else {
            return Err(ChonkitError::DoesNotExist(format!(
                "Collection with ID '{collection_id}'"
            )));
        };
        let vector_db = self.providers.vector.provider(&collection.provider)?;
        vector_db.count_vectors(&collection.name, document_id).await
    }
}

/// Vector service DTOs.
pub mod dto {
    use serde::Deserialize;
    use utoipa::ToSchema;
    use uuid::Uuid;
    use validify::{
        field_err, schema_err, schema_validation, ValidationError, ValidationErrors, Validify,
    };

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

    #[derive(Debug, Deserialize, Validify, ToSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateCollectionPayload {
        /// Collection name. Cannot contain special characters.
        #[validate(custom(ascii_alphanumeric_underscored))]
        #[validate(custom(begins_with_capital_ascii_letter))]
        #[validate(length(min = 1))]
        #[modify(trim)]
        pub name: String,

        /// Collection model.
        pub model: String,

        /// Vector database provider.
        pub vector_provider: String,

        /// Embeddings provider.
        pub embedding_provider: String,
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
    #[derive(Debug, Deserialize, Validify, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[validate(Self::validate_schema)]
    pub struct SearchPayload {
        /// The text to search by.
        #[modify(trim)]
        pub query: String,

        /// The collection to search in. Has priority over
        /// everything else.
        pub collection_id: Option<Uuid>,

        /// If given search via the name and provider combo.
        #[validate(length(min = 1))]
        #[modify(trim)]
        pub collection_name: Option<String>,

        /// Vector provider.
        pub provider: Option<String>,

        /// Amount of results to return.
        pub limit: Option<u32>,
    }

    impl SearchPayload {
        #[schema_validation]
        fn validate_schema(&self) -> Result<(), ValidationErrors> {
            let SearchPayload {
                collection_id,
                collection_name,
                provider,
                ..
            } = self;
            match (collection_id, collection_name, provider) {
                (None, None, None) => {
                    schema_err!(
                        "either_id_or_name_and_provider",
                        "one of either `collection_id`, or `provider` and `collection_name` combination must be set"
                    );
                }
                (None, Some(_), None) | (None, None, Some(_)) => {
                    schema_err!(
                    "name_and_provider",
                    "both 'collection_name'and 'provider' must be set if `collection_id` is not set"
                );
                }
                _ => {}
            }
        }
    }
}
