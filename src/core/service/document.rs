use crate::{
    core::{
        chunk::ChunkConfig,
        document::{
            parser::{ParseConfig, Parser},
            store::DocumentStore,
        },
        model::document::{Document, DocumentInsert, DocumentType},
        repo::document::DocumentRepo,
    },
    error::ChonkitError,
};
use uuid::Uuid;

/// # CORE
/// High level operations for document management.
#[derive(Debug, Clone)]
pub struct DocumentService<R, S> {
    pub repo: R,
    pub storage: S,
}

impl<R, S> DocumentService<R, S>
where
    R: DocumentRepo,
    S: DocumentStore,
{
    pub fn new(repo: R, storage: S) -> Self {
        Self { repo, storage }
    }

    /// Get the metadata for a document.
    ///
    /// * `id`: The ID of the document.
    pub async fn get_metadata(&self, id: Uuid) -> Result<Document, ChonkitError> {
        let file = self.repo.get_by_id(id).await?;

        let Some(file) = file else {
            return Err(ChonkitError::NotFound(id.to_string()));
        };

        Ok(file)
    }

    /// Get document content.
    ///
    /// * `path`: Where to read from.
    pub async fn get_content(&self, id: uuid::Uuid) -> Result<String, ChonkitError> {
        let document = self.repo.get_by_id(id).await?;

        let Some(document) = document else {
            return Err(ChonkitError::DoesNotExist(format!("Document with ID {id}")));
        };

        let config = self.repo.get_parse_config(document.id).await?;
        let ext = document.ext.as_str();

        let parser = match config {
            Some(cfg) => cfg.config,
            None => Parser::new(ext.try_into()?),
        };

        self.storage.read(&document, parser).await
    }

    /// Insert the document metadata to the repository and persist it
    /// in the underlying storage implementation.
    ///
    /// * `name`: Document name.
    /// * `ext`:  Document extension.
    /// * `file`: Document file.
    pub async fn upload(
        &self,
        name: &str,
        ty: DocumentType,
        file: &[u8],
    ) -> Result<Document, ChonkitError> {
        let path = self.storage.write(name, file).await?;
        let insert = DocumentInsert::new(name, &path, ty);
        let document = self.repo.insert(insert).await?;
        Ok(document)
    }

    pub async fn update_parser(
        &self,
        id: uuid::Uuid,
        config: ParseConfig,
    ) -> Result<(), ChonkitError> {
        todo!()
    }

    pub async fn update_chunker(
        &self,
        id: uuid::Uuid,
        config: ChunkConfig,
    ) -> Result<(), ChonkitError> {
        todo!()
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
            service::document::DocumentService,
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
        let name = "UPLOAD_TEST_TXT";
        let ext = DocumentType::Text;
        let content = b"Hello World!";

        let document = service.upload(name, ext, content).await.unwrap();

        let text_from_bytes = TextParser::default().parse(content).unwrap();
        let text_from_store = service.get_content(document.id).await.unwrap();

        assert_eq!(text_from_bytes, text_from_store);
    }

    #[test]
    async fn upload_pdf_happy(service: Service) {
        let name = "UPLOAD_TEST_PDF";
        let ext = DocumentType::Pdf;
        let content = tokio::fs::read("test_docs/test.pdf").await.unwrap();

        let document = service.upload(name, ext, &content).await.unwrap();

        let text_from_bytes = PdfParser::default().parse(&content).unwrap();
        let text_from_store = service.get_content(document.id).await.unwrap();

        assert_eq!(text_from_bytes, text_from_store);
    }

    #[test]
    async fn upload_docx_happy(service: Service) {
        let name = "UPLOAD_TEST_DOCX";
        let ext = DocumentType::Docx;
        let content = tokio::fs::read("test_docs/test.docx").await.unwrap();

        let document = service.upload(name, ext, &content).await.unwrap();

        let text_from_bytes = DocxParser::default().parse(&content).unwrap();
        let text_from_store = service.get_content(document.id).await.unwrap();

        assert_eq!(text_from_bytes, text_from_store);
    }
}
