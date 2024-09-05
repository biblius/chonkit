use crate::{
    core::{
        chunk::{Chunker, DocumentChunker},
        document::{
            parser::{ParseConfig, Parser},
            sha256,
            store::DocumentStore,
        },
        model::{
            document::{Document, DocumentConfig, DocumentInsert, DocumentType},
            List, Pagination,
        },
        repo::document::DocumentRepo,
    },
    error::ChonkitError,
};
use dto::ChunkPreviewPayload;
use tracing::info;
use uuid::Uuid;
use validify::{Validate, Validify};

pub mod dto;

/// High level operations for document management.
#[derive(Debug, Clone)]
pub struct DocumentService<R, S> {
    pub repo: R,
    pub storage: S,
}

impl<R, S> DocumentService<R, S>
where
    R: DocumentRepo + Sync,
    S: DocumentStore,
{
    pub fn new(repo: R, storage: S) -> Self {
        Self { repo, storage }
    }

    /// Get a paginated list of documents from the repository.
    ///
    /// * `p`: Pagination.
    pub async fn list_documents(&self, p: Pagination) -> Result<List<Document>, ChonkitError> {
        self.repo.list(p).await
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
        let document = self.repo.get_by_id(id).await?;

        let Some(document) = document else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };

        let ext = document.ext.as_str().try_into()?;
        let parser = self.get_parser(id, ext).await?;

        self.storage.read(&document, parser).await
    }

    /// Get document chunks using its parsing and chunking configuration.
    ///
    /// * `id`: Document ID.
    pub async fn get_chunks(&self, id: Uuid) -> Result<Vec<String>, ChonkitError> {
        let content = self.get_content(id).await?;
        let chunker = self
            .repo
            .get_chunk_config(id)
            .await?
            .map(|config| config.config)
            .unwrap_or_else(Chunker::snapping_default);

        Ok(chunker
            .chunk(&content)?
            .into_iter()
            .map(String::from)
            .collect())
    }

    /// Insert the document metadata to the repository and persist it
    /// in the underlying storage implementation.
    ///
    /// * `name`: Document name.
    /// * `ext`:  Document extension.
    /// * `file`: Document file.
    pub async fn upload(&self, mut params: DocumentUpload<'_>) -> Result<Document, ChonkitError> {
        params.validify()?;

        let DocumentUpload { ref name, ty, file } = params;
        let hash = sha256(file);

        let existing = self.repo.get_by_hash(&hash).await?;

        if let Some(Document { name: existing, .. }) = existing {
            return Err(ChonkitError::AlreadyExists(format!(
                "New document ({name}) has same hash as existing ({existing})"
            )));
        };

        let path = self.storage.write(name, file).await?;
        let insert = DocumentInsert::new(name, &path, ty, &hash);
        let document = self.repo.insert(insert).await?;
        Ok(document)
    }

    /// Remove the document from the repo and delete it from the storage.
    ///
    /// * `id`: Document ID.
    pub async fn delete(&self, id: Uuid) -> Result<(), ChonkitError> {
        let document = self.repo.get_by_id(id).await?;
        let Some(document) = document else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };
        self.repo.remove_by_id(document.id).await?;
        self.storage.delete(&document.path).await
    }

    /// Sync storage contents with the repo.
    pub async fn sync(&self) -> Result<(), ChonkitError> {
        self.storage.sync(&self.repo).await
    }

    /// Preview how the document gets parsed to text.
    ///
    /// * `id`: Document ID.
    /// * `config`: If given, uses the parsing config, otherwise use the default parser for the
    ///             file type.
    pub async fn parse_preview(
        &self,
        id: Uuid,
        config: Option<ParseConfig>,
    ) -> Result<String, ChonkitError> {
        let document = self.repo.get_by_id(id).await?;

        let Some(document) = document else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };

        if let Some(ref config) = config {
            config.validate()?;
        }

        let parser = Parser::new_from(
            document.ext.as_str().try_into()?,
            config.unwrap_or_default(),
        );

        self.storage.read(&document, parser).await
    }

    /// Chunk the document without saving any embeddings. Useful for previewing.
    /// If a chunker is given, it will be used. If no chunker is given, searches
    /// for the repo for a configured one. If it still doesn't exist, uses the default
    /// snapping window chunker.
    ///
    /// * `id`: Document ID.
    /// * `parse_config`: If given, use the config, otherwise use the configured parser or the
    ///                   default if it does not exist.
    /// * `chunker`: Optional chunker to use.
    pub async fn chunk_preview(
        &self,
        id: Uuid,
        payload: Option<ChunkPreviewPayload>,
    ) -> Result<Vec<String>, ChonkitError> {
        let document = self.repo.get_by_id(id).await?;
        let Some(document) = document else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };

        let ChunkPreviewPayload { parser, chunker } = payload.unwrap_or_default();

        let ext = document.ext.as_str().try_into()?;
        let parser = if let Some(config) = parser {
            Parser::new_from(ext, config)
        } else {
            self.get_parser(id, ext).await?
        };
        let content = self.storage.read(&document, parser).await?;
        let chunker = if let Some(chunker) = chunker {
            chunker
        } else {
            self.repo
                .get_chunk_config(id)
                .await?
                .map(|config| config.config)
                .unwrap_or_default()
        };

        info!("Chunking {} with {chunker}", document.name);

        Ok(chunker
            .chunk(&content)?
            .into_iter()
            .map(|chunk| chunk.to_owned())
            .collect())
    }

    pub async fn update_parser(&self, id: Uuid, config: ParseConfig) -> Result<(), ChonkitError> {
        config.validate()?;

        let document = self.repo.get_by_id(id).await?;

        if document.is_none() {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        }

        self.repo.upsert_parse_config(id, config).await?;

        Ok(())
    }

    pub async fn update_chunker(&self, id: Uuid, config: Chunker) -> Result<(), ChonkitError> {
        let document = self.repo.get_by_id(id).await?;

        if document.is_none() {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        }

        self.repo.upsert_chunk_config(id, config).await?;

        Ok(())
    }

    async fn get_parser(&self, id: Uuid, ext: DocumentType) -> Result<Parser, ChonkitError> {
        let config = self.repo.get_parse_config(id).await?;
        match config {
            Some(cfg) => Ok(Parser::new_from(ext, cfg.config)),
            None => Ok(Parser::new(ext)),
        }
    }
}

#[derive(Debug, Validify)]
pub struct DocumentUpload<'a> {
    #[modify(trim)]
    #[validate(length(min = 1, message = "Document name cannot be empty."))]
    pub name: String,
    pub ty: DocumentType,
    pub file: &'a [u8],
}

impl<'a> DocumentUpload<'a> {
    pub fn new(name: String, ty: DocumentType, file: &'a [u8]) -> Self {
        Self { name, ty, file }
    }
}

#[cfg(test)]
#[suitest::suite(service_doc_pg_fs_int)]
mod document_service_tests {
    use crate::{
        app::{
            document::store::FsDocumentStore,
            repo::pg::{document::PgDocumentRepo, init},
        },
        core::{
            document::parser::{
                docx::DocxParser, pdf::PdfParser, text::TextParser, DocumentParser,
            },
            model::document::DocumentType,
            service::document::{DocumentService, DocumentUpload},
        },
    };
    use suitest::{after_all, before_all, cleanup};

    const WORKING_DIR: &str = "__doc_service_tests";
    type Service = DocumentService<PgDocumentRepo, FsDocumentStore>;

    #[before_all]
    async fn setup() -> (PgDocumentRepo, FsDocumentStore, Service) {
        let url = std::env::var("DATABASE_URL").expect("no database url");
        let client = init(&url).await;

        tokio::fs::create_dir(WORKING_DIR).await.unwrap();

        let repo = PgDocumentRepo::new(client.clone());
        let store = FsDocumentStore::new(WORKING_DIR);
        let service = DocumentService::new(repo.clone(), store.clone());

        (repo, store, service)
    }

    #[after_all]
    async fn teardown() {
        tokio::fs::remove_dir_all(WORKING_DIR).await.unwrap();
    }

    #[cleanup]
    async fn cleanup() {
        tokio::fs::remove_dir_all(WORKING_DIR).await.unwrap();
    }

    #[test]
    async fn upload_text_happy(service: Service) {
        let content = b"Hello world";
        let upload = DocumentUpload {
            name: "UPLOAD_TEST_TXT".to_string(),
            ty: DocumentType::Text,
            file: content,
        };

        let document = service.upload(upload).await.unwrap();

        let text_from_bytes = TextParser::default().parse(content).unwrap();
        let text_from_store = service.get_content(document.id).await.unwrap();

        assert_eq!(text_from_bytes, text_from_store);

        service.delete(document.id).await.unwrap();
    }

    #[test]
    async fn upload_pdf_happy(service: Service) {
        let content = &tokio::fs::read("test_docs/test.pdf").await.unwrap();
        let upload = DocumentUpload {
            name: "UPLOAD_TEST_PDF".to_string(),
            ty: DocumentType::Pdf,
            file: content,
        };

        let document = service.upload(upload).await.unwrap();

        let text_from_bytes = PdfParser::default().parse(content).unwrap();
        let text_from_store = service.get_content(document.id).await.unwrap();

        assert_eq!(text_from_bytes, text_from_store);

        service.delete(document.id).await.unwrap();
    }

    #[test]
    async fn upload_docx_happy(service: Service) {
        let content = &tokio::fs::read("test_docs/test.docx").await.unwrap();
        let upload = DocumentUpload {
            name: "UPLOAD_TEST_DOCX".to_string(),
            ty: DocumentType::Docx,
            file: content,
        };

        let document = service.upload(upload).await.unwrap();

        let text_from_bytes = DocxParser::default().parse(content).unwrap();
        let text_from_store = service.get_content(document.id).await.unwrap();

        assert_eq!(text_from_bytes, text_from_store);

        service.delete(document.id).await.unwrap();
    }
}
