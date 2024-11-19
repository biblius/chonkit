#[cfg(test)]
#[suitest::suite(integration_tests)]
#[suitest::suite_cfg(sequential = true)]
mod document_service_integration_tests {
    use crate::{
        app::test::{TestState, TestStateConfig},
        core::{
            document::parser::{
                docx::DocxParser, pdf::PdfParser, text::TextParser, DocumentParser, ParseConfig,
            },
            model::document::DocumentType,
            service::document::dto::DocumentUpload,
        },
    };

    type DocumentService = crate::core::service::document::DocumentService<PgPool>;

    const TEST_UPLOAD_PATH: &str = "__document_service_test_upload__";
    const TEST_DOCS_PATH: &str = "test/docs";
    use sqlx::PgPool;
    use suitest::{after_all, before_all, cleanup};

    #[before_all]
    async fn setup() -> (TestState, DocumentService) {
        tokio::fs::create_dir(TEST_UPLOAD_PATH).await.unwrap();

        let test_state = TestState::init(TestStateConfig {
            fs_store_path: TEST_UPLOAD_PATH.to_string(),
        })
        .await;

        let service = test_state.app.services.document.clone();

        (test_state, service)
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

        let document = service.upload("fs", upload).await.unwrap();

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

        let document = service.upload("fs", upload).await.unwrap();

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

        let document = service.upload("fs", upload).await.unwrap();

        let text_from_bytes = DocxParser::default().parse(content).unwrap();
        let text_from_store = service.get_content(document.id).await.unwrap();

        assert_eq!(text_from_bytes, text_from_store);

        service.delete(document.id).await.unwrap();

        assert!(tokio::fs::metadata(document.path).await.is_err());
    }

    #[test]
    async fn update_parser(service: DocumentService) {
        let content = &tokio::fs::read(format!("{TEST_DOCS_PATH}/test.pdf"))
            .await
            .unwrap();

        let upload = DocumentUpload {
            name: "UPLOAD_TEST_PARSER".to_string(),
            ty: DocumentType::Pdf,
            file: content,
        };

        let document = service.upload("fs", upload).await.unwrap();

        let config = ParseConfig::new(10, 20)
            .use_range()
            .with_filter("foo")
            .unwrap();

        service
            .update_parser(document.id, config.clone())
            .await
            .unwrap();

        let document = service.get_config(document.id).await.unwrap();
        let parse_config = document.parse_config.unwrap();

        assert_eq!(config.start, parse_config.start);
        assert_eq!(config.end, parse_config.end);
        assert_eq!(
            config.filters[0].to_string(),
            parse_config.filters[0].to_string()
        );
        assert_eq!(config.range, parse_config.range);
    }
}
