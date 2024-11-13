use crate::{
    core::{
        chunk::{ChunkedDocument, Chunker},
        document::{
            parser::{ParseConfig, Parser},
            sha256,
            store::DocumentSync,
        },
        embedder::Embedder,
        model::{
            document::{Document, DocumentConfig, DocumentDisplay, DocumentInsert, DocumentType},
            List, PaginationSort,
        },
        provider::ProviderState,
        repo::{document::DocumentRepo, Atomic},
    },
    error::ChonkitError,
    transaction,
};
use dto::{ChunkPreviewPayload, DocumentUpload};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use validify::{Validate, Validify};

/// High level operations for document management.
#[derive(Clone)]
pub struct DocumentService<R> {
    pub repo: R,
    providers: ProviderState,
}

impl<R> DocumentService<R>
where
    R: DocumentRepo + Atomic + Send + Sync,
{
    pub fn new(repo: R, providers: ProviderState) -> Self {
        Self { repo, providers }
    }

    /// Get a paginated list of documents from the repository.
    ///
    /// * `p`: Pagination and sorting options.
    /// * `src`: Optional document source to filter by.
    /// * `ready`: If given and `true`, return only documents that are ready for processing.
    pub async fn list_documents(
        &self,
        p: PaginationSort,
        src: Option<&str>,
        ready: Option<bool>,
    ) -> Result<List<Document>, ChonkitError> {
        p.validate()?;
        self.repo.list(p, src, ready).await
    }

    /// Get a paginated list of documents from the repository with additional info for each.
    ///
    /// * `p`: Pagination.
    pub async fn list_documents_display(
        &self,
        p: PaginationSort,
        src: Option<&str>,
        document_id: Option<Uuid>,
    ) -> Result<List<DocumentDisplay>, ChonkitError> {
        p.validate()?;
        self.repo.list_with_collections(p, src, document_id).await
    }

    /// Get a document from the repository.
    ///
    /// * `id`: Document ID.
    pub async fn get_document(&self, id: Uuid) -> Result<Document, ChonkitError> {
        self.repo
            .get_by_id(id)
            .await?
            .ok_or_else(|| ChonkitError::DoesNotExist(format!("Document with ID '{id}'")))
    }

    /// Get the full config for a document.
    ///
    /// * `id`: Document ID.
    pub async fn get_config(&self, id: Uuid) -> Result<DocumentConfig, ChonkitError> {
        let file = self.repo.get_config_by_id(id).await?;

        let Some(file) = file else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };

        Ok(file)
    }

    /// Get document text content using its parsing configuration,
    /// or the default parser if it has no configuration.
    ///
    /// * `id`: Document ID.
    pub async fn get_content(&self, id: Uuid) -> Result<String, ChonkitError> {
        let Some(document) = self.repo.get_by_id(id).await? else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };

        let store = self.providers.store.get_provider(&document.src)?;

        let ext = document.ext.as_str().try_into()?;
        let parser = self.get_parser(id, ext).await?;

        store.read(&document, &parser).await
    }

    /// Get document chunks using its parsing and chunking configuration,
    /// or the default configurations if they have no configuration.
    ///
    /// * `document`: Document ID.
    /// * `content`: The document's content.
    pub async fn get_chunks<'content>(
        &self,
        document: &Document,
        content: &'content str,
    ) -> Result<ChunkedDocument<'content>, ChonkitError> {
        let mut chunker = self
            .repo
            .get_chunk_config(document.id)
            .await?
            .map(|config| config.config)
            .ok_or_else(|| {
                ChonkitError::DoesNotExist(format!(
                    "Chunking config for document with ID {}",
                    document.id
                ))
            })?;

        // If it's a semantic chunker, it needs an embedder.
        if let Chunker::Semantic(ref mut chunker) = chunker {
            let embedder = self
                .providers
                .embedding
                .get_provider(&chunker.config.embed_provider)?;
            chunker.embedder(embedder);
        }

        Ok(chunker.chunk(content).await?)
    }

    /// Insert the document metadata to the repository and persist it
    /// in the underlying storage implementation.
    ///
    /// * `store`: The storage implementation.
    /// * `params`: Upload params.
    pub async fn upload(
        &self,
        storage_provider: &str,
        mut params: DocumentUpload<'_>,
    ) -> Result<DocumentConfig, ChonkitError> {
        params.validify()?;

        let DocumentUpload { ref name, ty, file } = params;
        let hash = sha256(file);
        let store = self.providers.store.get_provider(storage_provider)?;

        let existing = self.repo.get_by_hash(&hash).await?;

        if let Some(Document { name: existing, .. }) = existing {
            return Err(ChonkitError::AlreadyExists(format!(
                "New document ({name}) has same hash as existing ({existing})"
            )));
        };

        transaction!(self.repo, |tx| async move {
            let path = store.write(name, file).await?;

            let insert = DocumentInsert::new(name, &path, ty, &hash, store.id());
            let parse_config = ParseConfig::default();
            let chunk_config = Chunker::snapping_default();

            let document = self
                .repo
                .insert_with_configs(insert, parse_config, chunk_config, tx)
                .await?;

            Ok(document)
        })
    }

    /// Remove the document from the repo and delete it from the storage.
    ///
    /// * `id`: Document ID.
    pub async fn delete(&self, id: Uuid) -> Result<(), ChonkitError> {
        let Some(document) = self.repo.get_by_id(id).await? else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };
        let store = self.providers.store.get_provider(&document.src)?;
        self.repo.remove_by_id(document.id).await?;
        store.delete(&document.path).await
    }

    /// Sync storage contents with the repo.
    pub async fn sync<T>(&self, store: &T) -> Result<(), ChonkitError>
    where
        T: DocumentSync<R> + Send + Sync + ?Sized,
    {
        store.sync(&self.repo).await
    }

    pub async fn chunk_preview(
        &self,
        document_id: Uuid,
        config: ChunkPreviewPayload,
    ) -> Result<Vec<String>, ChonkitError> {
        config.validate()?;

        let embedder = if let Some(embedder) = &config.embedder {
            Some(self.providers.embedding.get_provider(embedder.as_str())?)
        } else {
            None
        };

        let parser = if let Some(parser) = config.parser {
            parser
        } else {
            let config = self.get_config(document_id).await?;
            config.parse_config.ok_or_else(|| {
                ChonkitError::DoesNotExist(format!("Parsing configuration for {document_id}"))
            })?
        };

        let content = self.parse_preview(document_id, parser).await?;
        let chunked = self
            .chunk_preview_inner(&content, config.chunker, embedder)
            .await?;

        match chunked {
            ChunkedDocument::Ref(chunked) => Ok(chunked.into_iter().map(String::from).collect()),
            ChunkedDocument::Owned(chunked) => Ok(chunked),
        }
    }

    /// Preview how the document gets parsed to text.
    ///
    /// * `id`: Document ID.
    /// * `config`: If given, uses the parsing config, otherwise use the default parser for the
    ///             file type.
    pub async fn parse_preview(
        &self,
        id: Uuid,
        config: ParseConfig,
    ) -> Result<String, ChonkitError> {
        config.validate()?;

        let document = self.repo.get_by_id(id).await?;

        let Some(document) = document else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };

        let store = self.providers.store.get_provider(&document.src)?;

        let ext = document.ext.as_str().try_into()?;
        let parser = Parser::new_from(ext, config);

        info!("Using parser ({ext}) for '{id}'");

        store.read(&document, &parser).await
    }

    /// Update a document's parsing configuration.
    ///
    /// * `id`: Document ID.
    /// * `config`: Parsing configuration.
    pub async fn update_parser(&self, id: Uuid, config: ParseConfig) -> Result<(), ChonkitError> {
        config.validate()?;

        let document = self.repo.get_by_id(id).await?;

        if document.is_none() {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        }

        self.repo.upsert_parse_config(id, config).await?;

        Ok(())
    }

    /// Update a document's chunking configuration.
    ///
    /// * `id`: Document ID.
    /// * `config`: Chunking configuration.
    pub async fn update_chunker(&self, id: Uuid, config: Chunker) -> Result<(), ChonkitError> {
        let document = self.repo.get_by_id(id).await?;

        if document.is_none() {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        }

        self.repo.upsert_chunk_config(id, config).await?;

        Ok(())
    }

    /// Chunk the document without saving any embeddings. Useful for previewing.
    /// If a chunker is given, it will be used. If no chunker is given, searches
    /// for the repo for a configured one. If it still doesn't exist, uses the default
    /// snapping window chunker.
    ///
    /// * `content`: Document content.
    /// * `chunker`: The chunker to chunk the content with.
    /// * `embedder`: The embedder to use for semantic chunking. Required only when using the
    ///               semantic chunker.
    async fn chunk_preview_inner<'content>(
        &self,
        content: &'content str,
        mut chunker: Chunker,
        embedder: Option<Arc<dyn Embedder + Send + Sync>>,
    ) -> Result<ChunkedDocument<'content>, ChonkitError> {
        // If it's a semantic chunker, it needs an embedder.
        if let Chunker::Semantic(ref mut chunker) = chunker {
            let Some(embedder) = embedder else {
                return Err(ChonkitError::InvalidEmbeddingModel(
                    "No embedder provided for semantic chunker".to_string(),
                ));
            };
            chunker.embedder(embedder);
        }
        Ok(chunker.chunk(&content).await?)
    }

    /// Get a parser for a document, or a default parser if the document has no configuration.
    ///
    /// * `id`: Document ID.
    /// * `ext`: Document extension.
    async fn get_parser(&self, id: Uuid, ext: DocumentType) -> Result<Parser, ChonkitError> {
        let config = self.repo.get_parse_config(id).await?;
        match config {
            Some(cfg) => Ok(Parser::new_from(ext, cfg.config)),
            None => Ok(Parser::new(ext)),
        }
    }
}

/// Document service DTOs.
pub mod dto {
    use crate::core::{
        chunk::Chunker, document::parser::ParseConfig, model::document::DocumentType,
    };
    use serde::Deserialize;
    use validify::{schema_err, schema_validation, Validate, ValidationErrors, Validify};

    #[derive(Debug, Validify)]
    pub struct DocumentUpload<'a> {
        /// Document name.
        #[modify(trim)]
        #[validate(length(min = 1, message = "Document name cannot be empty."))]
        pub name: String,

        /// Document extension.
        pub ty: DocumentType,

        /// Document file.
        pub file: &'a [u8],
    }

    impl<'a> DocumentUpload<'a> {
        pub fn new(name: String, ty: DocumentType, file: &'a [u8]) -> Self {
            Self { name, ty, file }
        }
    }

    /// DTO used for previewing chunks.
    #[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[validate(Self::validate_schema)]
    pub struct ChunkPreviewPayload {
        /// Parsing configuration.
        pub parser: Option<ParseConfig>,

        /// Chunking configuration.
        pub chunker: Chunker,

        /// The embedding provider to use. Necessary
        /// when using the semantic chunker.
        pub embedder: Option<String>,
    }

    impl ChunkPreviewPayload {
        #[schema_validation]
        fn validate_schema(&self) -> Result<(), ValidationErrors> {
            if let (Chunker::Semantic(_), None) = (&self.chunker, &self.embedder) {
                schema_err! {
                    "chunker_params",
                    "`embedder` must be set when using semantic chunker"
                };
            }
        }
    }
}
