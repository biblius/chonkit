// Tests vector service integration depending on the features used.
#[cfg(all(
    test,
    all(any(feature = "qdrant", feature = "weaviate"), feature = "fe-local")
))]
#[suitest::suite(integration_tests)]
mod vector_service_integration_tests {
    use crate::{
        app::test::{TestState, TestStateConfig},
        config::DEFAULT_COLLECTION_NAME,
        core::{
            model::document::{DocumentInsert, DocumentType, TextDocumentType},
            provider::ProviderFactory,
            repo::{document::DocumentRepo, vector::VectorRepo},
            service::vector::dto::{CreateCollectionPayload, CreateEmbeddings, SearchPayload},
        },
        error::ChonkitErr,
    };
    use suitest::{after_all, before_all, cleanup};

    const TEST_UPLOAD_PATH: &str = "__vector_service_test_upload__";

    #[before_all]
    async fn setup() -> TestState {
        let _ = tokio::fs::remove_dir_all(TEST_UPLOAD_PATH).await;
        tokio::fs::create_dir(TEST_UPLOAD_PATH).await.unwrap();

        let test_state = TestState::init(TestStateConfig {
            fs_store_path: TEST_UPLOAD_PATH.to_string(),
        })
        .await;

        for provider in test_state.active_vector_providers.iter() {
            test_state
                .app
                .services
                .vector
                .create_default_collection(
                    provider,
                    test_state
                        .app
                        .providers
                        .embedding
                        .get_provider("fembed")
                        .unwrap()
                        .id(),
                )
                .await;
        }

        test_state
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
    async fn default_collection_stored_successfully(state: TestState) {
        let service = &state.app.services.vector;
        let embedder = state
            .app
            .providers
            .embedding
            .get_provider("fembed")
            .unwrap()
            .clone();

        for provider in state.active_vector_providers.iter() {
            let vector_db = state.app.providers.vector.get_provider(provider).unwrap();
            let collection_name =
                format!("{DEFAULT_COLLECTION_NAME}_{}_{}", provider, embedder.id());

            let collection = state
                .app
                .services
                .vector
                .get_collection_by_name(&collection_name, provider)
                .await
                .unwrap();

            assert_eq!(collection.name, collection_name);
            assert_eq!(collection.model, embedder.default_model().0);
            assert_eq!(collection.embedder, embedder.id());
            assert_eq!(collection.provider, *provider);

            let v_collection = vector_db.get_collection(&collection_name).await.unwrap();

            let size = embedder.size(&collection.model).await.unwrap().unwrap();

            assert_eq!(size, v_collection.size);

            // Assert this can be called again without errors.
            service
                .create_default_collection(vector_db.id(), embedder.id())
                .await;
        }
    }

    #[test]
    async fn create_collection_works(state: TestState) {
        let service = &state.app.services.vector;
        let embedder = state
            .app
            .providers
            .embedding
            .get_provider("fembed")
            .unwrap()
            .clone();

        for provider in state.active_vector_providers.iter() {
            let vector_db = state.app.providers.vector.get_provider(provider).unwrap();

            let name = "Test_collection_0";
            let model = embedder
                .list_embedding_models()
                .await
                .unwrap()
                .first()
                .cloned()
                .unwrap();

            let params = CreateCollectionPayload {
                model: model.0.clone(),
                name: name.to_string(),
                vector_provider: vector_db.id().to_string(),
                embedding_provider: embedder.id().to_string(),
            };

            let collection = service.create_collection(params).await.unwrap();

            assert_eq!(collection.name, name);
            assert_eq!(collection.model, model.0);
            assert_eq!(collection.embedder, embedder.id());
            assert_eq!(collection.provider, vector_db.id());

            let v_collection = vector_db.get_collection(name).await.unwrap();

            let size = embedder.size(&collection.model).await.unwrap().unwrap();

            assert_eq!(size, v_collection.size);
        }
    }

    #[test]
    async fn create_collection_fails_with_invalid_model(state: TestState) {
        let service = &state.app.services.vector;
        let embedder = state
            .app
            .providers
            .embedding
            .get_provider("fembed")
            .unwrap()
            .clone();

        for provider in state.active_vector_providers.iter() {
            let vector_db = state.app.providers.vector.get_provider(provider).unwrap();

            let name = "Test_collection_0";

            let params = CreateCollectionPayload {
                model: "invalid_model".to_string(),
                name: name.to_string(),
                vector_provider: vector_db.id().to_string(),
                embedding_provider: embedder.id().to_string(),
            };

            let result = service.create_collection(params).await;

            assert!(result.is_err());
        }
    }

    #[test]
    async fn create_collection_fails_with_existing_collection(state: TestState) {
        let service = &state.app.services.vector;
        let embedder = state
            .app
            .providers
            .embedding
            .get_provider("fembed")
            .unwrap()
            .clone();

        for provider in state.active_vector_providers.iter() {
            let vector_db = state.app.providers.vector.get_provider(provider).unwrap();
            let collection_name =
                format!("{DEFAULT_COLLECTION_NAME}_{}_{}", provider, embedder.id());

            let params = CreateCollectionPayload {
                model: embedder.default_model().0,
                name: collection_name,
                vector_provider: vector_db.id().to_string(),
                embedding_provider: embedder.id().to_string(),
            };

            let result = service.create_collection(params).await;

            assert!(result.is_err());
        }
    }

    #[test]
    async fn inserting_and_searching_embeddings_works(state: TestState) {
        let service = &state.app.services.vector;
        let postgres = &state.app.providers.database;
        let embedder = state
            .app
            .providers
            .embedding
            .get_provider("fembed")
            .unwrap()
            .clone();

        for provider in state.active_vector_providers.iter() {
            let vector_db = state.app.providers.vector.get_provider(provider).unwrap();
            let collection_name =
                format!("{DEFAULT_COLLECTION_NAME}_{}_{}", provider, embedder.id());

            let default = service
                .get_collection_by_name(&collection_name, vector_db.id())
                .await
                .unwrap();

            let create = DocumentInsert::new(
                "test_document",
                "test_path_1",
                DocumentType::Text(TextDocumentType::Txt),
                "SHA256_1",
                "fs",
            );

            let document = postgres.insert(create).await.unwrap();

            let content = r#"Hello World!"#;

            let embeddings = CreateEmbeddings {
                document_id: document.id,
                collection_id: default.id,
                chunks: &[content],
            };

            let collection = service
                .get_collection_by_name(&collection_name, vector_db.id())
                .await
                .unwrap();

            service.create_embeddings(embeddings).await.unwrap();

            let search = SearchPayload {
                query: content.to_string(),
                collection_id: Some(collection.id),
                limit: Some(1),
                collection_name: None,
                provider: None,
            };

            let results = service.search(search).await.unwrap();

            assert_eq!(1, results.len());
            assert_eq!(content, results[0]);

            let embeddings = postgres
                .get_embeddings_by_name(document.id, &collection_name, vector_db.id())
                .await
                .unwrap()
                .unwrap();

            let collection = postgres
                .get_collection(embeddings.collection_id)
                .await
                .unwrap()
                .unwrap();

            assert_eq!(collection.name, collection_name);
            assert_eq!(document.id, embeddings.document_id);

            let amount = postgres.remove_by_id(document.id, None).await.unwrap();
            assert_eq!(1, amount);
        }
    }

    #[test]
    async fn deleting_collection_removes_all_embeddings(state: TestState) {
        let service = &state.app.services.vector;
        let postgres = &state.app.providers.database;
        let embedder = state
            .app
            .providers
            .embedding
            .get_provider("fembed")
            .unwrap()
            .clone();

        for provider in state.active_vector_providers.iter() {
            let vector_db = state.app.providers.vector.get_provider(provider).unwrap();

            let collection_name = "Test_collection_delete_embeddings";

            let create = CreateCollectionPayload {
                name: collection_name.to_string(),
                model: embedder.default_model().0,
                vector_provider: vector_db.id().to_string(),
                embedding_provider: embedder.id().to_string(),
            };

            let collection = service.create_collection(create).await.unwrap();

            let create = DocumentInsert::new(
                "test_document",
                "test_path_2",
                DocumentType::Text(TextDocumentType::Txt),
                "SHA256_2",
                "fs",
            );

            let document = postgres.insert(create).await.unwrap();

            let content = r#"Hello World!"#;

            let embeddings = CreateEmbeddings {
                document_id: document.id,
                collection_id: collection.id,
                chunks: &[content],
            };

            service.create_embeddings(embeddings).await.unwrap();

            service.delete_collection(collection.id).await.unwrap();

            let embeddings = postgres
                .get_embeddings(document.id, collection.id)
                .await
                .unwrap();

            assert!(embeddings.is_none());

            let amount = postgres.remove_by_id(document.id, None).await.unwrap();
            assert_eq!(1, amount);
        }
    }

    #[test]
    async fn prevents_duplicate_embeddings(state: TestState) {
        let service = &state.app.services.vector;
        let postgres = &state.app.providers.database;
        let embedder = state
            .app
            .providers
            .embedding
            .get_provider("fembed")
            .unwrap()
            .clone();

        for provider in state.active_vector_providers.iter() {
            let vector_db = state.app.providers.vector.get_provider(provider).unwrap();
            let collection_name =
                format!("{DEFAULT_COLLECTION_NAME}_{}_{}", provider, embedder.id());

            let create = DocumentInsert::new(
                "test_document",
                "test_path_3",
                DocumentType::Text(TextDocumentType::Txt),
                "SHA256_3",
                "fs",
            );

            let default = service
                .get_collection_by_name(&collection_name, vector_db.id())
                .await
                .unwrap();

            let document = postgres.insert(create).await.unwrap();

            let content = r#"Hello World!"#;
            let create = CreateEmbeddings {
                document_id: document.id,
                collection_id: default.id,
                chunks: &[content],
            };

            service.create_embeddings(create.clone()).await.unwrap();

            let duplicate = service.create_embeddings(create).await;
            let error = duplicate.unwrap_err().error;

            assert!(matches!(error, ChonkitErr::AlreadyExists(_)));

            let amount = postgres.remove_by_id(document.id, None).await.unwrap();
            assert_eq!(1, amount);
        }
    }
}
