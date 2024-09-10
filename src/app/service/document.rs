use sqlx::PgPool;

use crate::{
    app::document::store::FsDocumentStore, core::service::document::DocumentService as Service,
};

pub(in crate::app) type DocumentService = Service<PgPool, FsDocumentStore>;

#[cfg(test)]
#[suitest::suite(integration_tests)]
mod document_service_tests {
    use crate::{
        app::{
            document::store::FsDocumentStore,
            service::document::DocumentService,
            test::{init_postgres, PostgresContainer},
        },
        core::{
            document::parser::{
                docx::DocxParser, pdf::PdfParser, text::TextParser, DocumentParser,
            },
            model::document::DocumentType,
            service::document::dto::DocumentUpload,
        },
    };

    const TEST_UPLOAD_PATH: &str = "__document_service_test_upload__";
    const TEST_DOCS_PATH: &str = "test/docs";
    use sqlx::PgPool;
    use suitest::{after_all, before_all, cleanup};

    #[before_all]
    async fn setup() -> (PgPool, FsDocumentStore, DocumentService, PostgresContainer) {
        tokio::fs::create_dir(TEST_UPLOAD_PATH).await.unwrap();

        let (client, _pg_img) = init_postgres().await;

        let store = FsDocumentStore::new(TEST_UPLOAD_PATH);
        let service = DocumentService::new(client.clone(), store.clone());

        (client, store, service, _pg_img)
    }

    #[cleanup]
    async fn cleanup() {
        let _ = tokio::fs::remove_dir_all(TEST_UPLOAD_PATH).await;
    }

    #[after_all]
    async fn teardown() {
        let _ = tokio::fs::remove_dir_all(TEST_UPLOAD_PATH).await;
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
        let content = &tokio::fs::read(format!("{TEST_DOCS_PATH}/test.pdf"))
            .await
            .unwrap();
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
        let content = &tokio::fs::read(format!("{TEST_DOCS_PATH}/test.docx"))
            .await
            .unwrap();
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
