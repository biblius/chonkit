use crate::{
    app::{document::store::FsDocumentStore, repo::pg::document::PgDocumentRepo},
    core::service::document::DocumentService as Service,
};

pub(in crate::app) type DocumentService = Service<PgDocumentRepo, FsDocumentStore>;

#[cfg(test)]
#[suitest::suite(integration_tests)]
mod document_service_postgres_fs {
    use crate::{
        app::{
            document::store::FsDocumentStore, repo::pg::document::PgDocumentRepo,
            service::document::DocumentService, test::init_postgres,
        },
        core::{
            document::parser::{
                docx::DocxParser, pdf::PdfParser, text::TextParser, DocumentParser,
            },
            model::document::DocumentType,
            service::document::dto::DocumentUpload,
        },
    };
    use suitest::{after_all, before_all, cleanup};
    use testcontainers::ContainerAsync;
    use testcontainers_modules::postgres::Postgres;

    const WORKING_DIR: &str = "__doc_service_tests";

    #[before_all]
    async fn setup() -> (
        PgDocumentRepo,
        FsDocumentStore,
        DocumentService,
        ContainerAsync<Postgres>,
    ) {
        let (client, _pg_img) = init_postgres().await;

        tokio::fs::create_dir(WORKING_DIR).await.unwrap();

        let repo = PgDocumentRepo::new(client);
        let store = FsDocumentStore::new(WORKING_DIR);
        let service = DocumentService::new(repo.clone(), store.clone());

        (repo, store, service, _pg_img)
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
    async fn upload_text_happy(service: DocumentService) {
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

        assert!(tokio::fs::metadata(document.path).await.is_err());
    }

    #[test]
    async fn upload_pdf_happy(service: DocumentService) {
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

        assert!(tokio::fs::metadata(document.path).await.is_err());
    }

    #[test]
    async fn upload_docx_happy(service: DocumentService) {
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

        assert!(tokio::fs::metadata(document.path).await.is_err());
    }
}
