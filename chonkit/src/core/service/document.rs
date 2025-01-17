use std::time::Instant;

use crate::{
    config::{DEFAULT_DOCUMENT_CONTENT, DEFAULT_DOCUMENT_NAME},
    core::{
        chunk::{
            ChunkConfig, ChunkedDocument, SemanticEmbedder, SemanticWindowConfig,
            SnappingWindowConfig,
        },
        document::{
            parser::{ParseConfig, Parser},
            sha256,
        },
        model::{
            document::{
                Document, DocumentConfig, DocumentDisplay, DocumentInsert, DocumentType,
                TextDocumentType,
            },
            List, Pagination, PaginationSort,
        },
        provider::ProviderState,
        repo::{document::DocumentRepo, Atomic},
    },
    err,
    error::ChonkitError,
    map_err, transaction,
};
use dto::{ChunkPreviewPayload, DocumentUpload};
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
        map_err!(p.validate());
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
        map_err!(p.validate());
        self.repo.list_with_collections(p, src, document_id).await
    }

    /// Get a document from the repository.
    ///
    /// * `id`: Document ID.
    pub async fn get_document(&self, id: Uuid) -> Result<Document, ChonkitError> {
        match self.repo.get_by_id(id).await? {
            Some(doc) => Ok(doc),
            None => err!(DoesNotExist, "Document with ID {id}"),
        }
    }

    /// Get the full config for a document.
    ///
    /// * `id`: Document ID.
    pub async fn get_config(&self, id: Uuid) -> Result<DocumentConfig, ChonkitError> {
        let file = self.repo.get_config_by_id(id).await?;

        let Some(file) = file else {
            return err!(DoesNotExist, "Document with ID {id}");
        };

        Ok(file)
    }

    /// Get document text content using its parsing configuration,
    /// or the default parser if it has no configuration.
    ///
    /// * `id`: Document ID.
    pub async fn get_content(&self, id: Uuid) -> Result<String, ChonkitError> {
        let Some(document) = self.repo.get_by_id(id).await? else {
            return err!(DoesNotExist, "Document with ID {id}");
        };

        let store = self.providers.document.get_provider(&document.src)?;

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
        let Some(config) = self
            .repo
            .get_chunk_config(document.id)
            .await?
            .map(|config| config.config)
        else {
            return err!(
                DoesNotExist,
                "Chunking config for document with ID {}",
                document.id
            );
        };

        self.chunk(config, content).await
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
        map_err!(params.validify());

        let DocumentUpload { ref name, ty, file } = params;
        let hash = sha256(file);
        let store = self.providers.document.get_provider(storage_provider)?;

        let existing = self.repo.get_by_hash(&hash).await?;

        if let Some(Document { name: existing, .. }) = existing {
            return err!(
                AlreadyExists,
                "New document ({name}) has same hash as existing ({existing})"
            );
        };

        transaction!(self.repo, |tx| async move {
            let path = store.write(name, file).await?;

            let insert = DocumentInsert::new(name, &path, ty, &hash, store.id());
            let parse_config = ParseConfig::default();
            let chunk_config = ChunkConfig::snapping_default();

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
            return err!(DoesNotExist, "Document with ID {id}");
        };
        let collections = self.repo.get_assigned_collection_names(document.id).await?;
        transaction! {self.repo, |tx| async move {
                self.repo.remove_by_id(document.id, Some(tx)).await?;
                for (collection, provider) in collections {
                    let vector_db = self.providers.vector.get_provider(&provider)?;
                    vector_db
                        .delete_embeddings(&collection, document.id)
                        .await?;
                }
                let store = self.providers.document.get_provider(&document.src)?;
                store.delete(&document.path).await
            }
        }
    }

    /// Sync storage contents with the repo.
    pub async fn sync(&self, storage_provider: &str) -> Result<(), ChonkitError> {
        let __start = Instant::now();
        let store = self.providers.document.get_provider(storage_provider)?;
        info!("Syncing documents with {}", store.id());

        // Prune
        let documents = self
            .repo
            .list(
                PaginationSort::new_default_sort(Pagination::new(10_000, 1)),
                Some(store.id()),
                None,
            )
            .await?;

        let non_existing = store.filter_non_existing(&documents.items).await?;

        for document_id in non_existing {
            self.repo.remove_by_id(document_id, None).await?;
        }

        // Store
        let files = store.list_files().await?;

        for file in files {
            let content = store.get_bytes(&file.path).await?;
            let hash = sha256(&content);

            let doc = self.repo.get_by_path(&file.path, store.id()).await?;

            if let Some(Document { id, name, .. }) = doc {
                info!("Document '{name}' already exists ({id})");
                continue;
            }

            let insert = DocumentInsert::new(&file.name, &file.path, file.ext, &hash, store.id());

            match self.repo.insert(insert).await {
                Ok(Document { id, name, .. }) => info!("Successfully inserted '{name}' ({id})"),
                Err(e) => tracing::error!("{e}"),
            }
        }

        info!(
            "Syncing finished for storage '{}', took {}ms",
            store.id(),
            Instant::now().duration_since(__start).as_millis()
        );

        Ok(())
    }

    /// Chunk the document without saving any embeddings. Useful for previewing.
    ///
    /// * `document_id`: ID of the document to chunk.
    /// * `config`: Chunking configuration.
    pub async fn chunk_preview(
        &self,
        document_id: Uuid,
        config: ChunkPreviewPayload,
    ) -> Result<Vec<String>, ChonkitError> {
        map_err!(config.validate());

        let parser = if let Some(parser) = config.parser {
            parser
        } else {
            let config = self.get_config(document_id).await?;
            match config.parse_config {
                Some(config) => config,
                None => return err!(DoesNotExist, "Parsing configuration for {document_id}"),
            }
        };

        let content = self.parse_preview(document_id, parser).await?;

        let chunks = match self.chunk(config.chunker, &content).await? {
            ChunkedDocument::Ref(chunked) => chunked.iter().map(|s| s.to_string()).collect(),
            ChunkedDocument::Owned(chunked) => chunked,
        };

        Ok(chunks)
    }

    async fn chunk<'i>(
        &self,
        config: ChunkConfig,
        input: &'i str,
    ) -> Result<ChunkedDocument<'i>, ChonkitError> {
        let chunks = match config {
            ChunkConfig::Sliding(config) => {
                let chunker = map_err!(chunx::SlidingWindow::new(config.size, config.overlap));
                let chunked = map_err!(chunker.chunk(input));

                ChunkedDocument::Ref(chunked)
            }
            ChunkConfig::Snapping(config) => {
                let SnappingWindowConfig {
                    size,
                    overlap,
                    delimiter,
                    skip_f,
                    skip_b,
                } = config;

                let chunker = map_err!(chunx::SnappingWindow::new(
                    size, overlap, delimiter, skip_f, skip_b
                ));

                let chunked = map_err!(chunker.chunk(input));

                ChunkedDocument::Owned(chunked)
            }
            ChunkConfig::Semantic(config) => {
                let SemanticWindowConfig {
                    size,
                    threshold,
                    distance_fn,
                    delimiter,
                    embedding_provider,
                    embedding_model,
                    skip_f,
                    skip_b,
                } = config;

                let chunker = chunx::SemanticWindow::new(
                    size,
                    threshold,
                    distance_fn,
                    delimiter,
                    skip_f,
                    skip_b,
                );

                let embedder = self.providers.embedding.get_provider(&embedding_provider)?;

                if embedder.size(&embedding_model).await?.is_none() {
                    return err!(
                        InvalidEmbeddingModel,
                        "Model '{embedding_model}' not supported by '{embedding_provider}'"
                    );
                };

                let semantic_embedder = SemanticEmbedder(embedder.clone());

                let chunked = chunker
                    .chunk(input, &semantic_embedder, &embedding_model)
                    .await?;

                ChunkedDocument::Owned(chunked)
            }
        };

        if chunks.is_empty() {
            return err!(Chunks, "chunks cannot be empty");
        }

        Ok(chunks)
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
        map_err!(config.validate());

        let document = self.repo.get_by_id(id).await?;

        let Some(document) = document else {
            return err!(DoesNotExist, "Document with ID {id}");
        };

        let store = self.providers.document.get_provider(&document.src)?;

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
        map_err!(config.validate());

        let document = self.repo.get_by_id(id).await?;

        if document.is_none() {
            return err!(DoesNotExist, "Document with ID {id}");
        }

        self.repo.upsert_parse_config(id, config).await?;

        Ok(())
    }

    /// Update a document's chunking configuration.
    ///
    /// * `id`: Document ID.
    /// * `config`: Chunking configuration.
    pub async fn update_chunker(&self, id: Uuid, config: ChunkConfig) -> Result<(), ChonkitError> {
        let document = self.repo.get_by_id(id).await?;

        if document.is_none() {
            return err!(DoesNotExist, "Document with ID {id}");
        }

        self.repo.upsert_chunk_config(id, config).await?;

        Ok(())
    }

    /// Creates the default document if no other document exists.
    pub async fn create_default_document(&self) {
        let count = self.repo.get_document_count().await.unwrap_or(0);

        if count > 0 {
            tracing::info!("Found existing documents, skipping default document creation");
            return;
        }

        match self
            .upload(
                "fs",
                DocumentUpload::new(
                    String::from(DEFAULT_DOCUMENT_NAME),
                    DocumentType::Text(TextDocumentType::Txt),
                    DEFAULT_DOCUMENT_CONTENT.as_bytes(),
                ),
            )
            .await
        {
            Ok(_) => tracing::info!("Created default document '{DEFAULT_DOCUMENT_NAME}'"),
            Err(e) => {
                if let crate::error::ChonkitErr::AlreadyExists(_) = e.error {
                    tracing::info!("Default document '{DEFAULT_DOCUMENT_NAME}' already exists");
                }
            }
        }
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
        chunk::ChunkConfig, document::parser::ParseConfig, model::document::DocumentType,
    };
    use serde::Deserialize;
    use validify::{Validate, Validify};

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
    pub struct ChunkPreviewPayload {
        /// Parsing configuration.
        pub parser: Option<ParseConfig>,

        /// Chunking configuration.
        pub chunker: ChunkConfig,
    }
}
