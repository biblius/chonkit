// Tests vector service integration depending on the features used.
#[cfg(all(test, any(feature = "qdrant", feature = "weaviate")))]
#[suitest::suite(integration_tests)]
mod vector_service_integration_tests {

    use crate::{
        app::{
            embedder::fastembed::FastEmbedder,
            test::{init_postgres, PostgresContainer},
        },
        core::{
            embedder::Embedder as _,
            model::document::{DocumentInsert, DocumentType},
            repo::{document::DocumentRepo, vector::VectorRepo},
            service::vector::dto::{CreateCollection, CreateEmbeddings, Search},
            vector::VectorDb,
        },
        error::ChonkitError,
        DEFAULT_COLLECTION_NAME,
    };
    use sqlx::PgPool;
    use suitest::before_all;
    use testcontainers::{ContainerAsync, GenericImage};

    type VectorService = crate::core::service::vector::VectorService<PgPool>;

    #[cfg(all(feature = "qdrant", feature = "weaviate"))]
    compile_error!("test can only be run with exactly 1 vector provider");

    #[cfg(feature = "qdrant")]
    type VectorDatabase = crate::app::vector::qdrant::QdrantDb;

    #[cfg(feature = "weaviate")]
    type VectorDatabase = crate::app::vector::weaviate::WeaviateDb;

    #[before_all]
    async fn setup() -> (
        PgPool,
        VectorDatabase,
        VectorService,
        FastEmbedder,
        PostgresContainer,
        ContainerAsync<GenericImage>,
    ) {
        let (postgres, pg) = init_postgres().await;

        #[cfg(feature = "qdrant")]
        let (vector_client, v_img) = crate::app::test::init_qdrant().await;

        #[cfg(feature = "weaviate")]
        let (vector_client, v_img) = crate::app::test::init_weaviate().await;

        #[cfg(feature = "fembed")]
        let embedder = crate::app::embedder::fastembed::FastEmbedder::new();

        let service = VectorService::new(postgres.clone());

        service
            .create_default_collection(&vector_client, &embedder)
            .await;

        (postgres, vector_client, service, embedder, pg, v_img)
    }

    #[test]
    async fn default_collection_is_stored_in_repo(
        service: VectorService,
        embedder: FastEmbedder,
        vector_db: VectorDatabase,
    ) {
        let collection = service
            .get_collection_by_name(DEFAULT_COLLECTION_NAME, vector_db.id())
            .await
            .unwrap();

        assert_eq!(collection.name, DEFAULT_COLLECTION_NAME);
        assert_eq!(collection.model, embedder.default_model().0);
        assert_eq!(collection.embedder, embedder.id());
        assert_eq!(collection.provider, vector_db.id());
    }

    #[test]
    async fn default_collection_is_stored_vec_db(
        service: VectorService,
        embedder: FastEmbedder,
        vector_db: VectorDatabase,
    ) {
        let collection = service
            .get_collection_by_name(DEFAULT_COLLECTION_NAME, vector_db.id())
            .await
            .unwrap();

        let v_collection = vector_db
            .get_collection(DEFAULT_COLLECTION_NAME)
            .await
            .unwrap();

        let size = embedder.size(&collection.model).await.unwrap().unwrap();

        assert_eq!(size, v_collection.size);

        // Assert this can be called again without errors.
        service.create_default_collection(vector_db, embedder).await;
    }

    #[test]
    async fn create_collection_works(
        service: VectorService,
        embedder: FastEmbedder,
        vector_db: VectorDatabase,
    ) {
        let name = "test_collection_0";
        let model = embedder
            .list_embedding_models()
            .await
            .unwrap()
            .first()
            .cloned()
            .unwrap();

        let params = CreateCollection {
            model: model.0.clone(),
            name: name.to_string(),
        };

        let collection = service
            .create_collection(vector_db, embedder, params)
            .await
            .unwrap();

        assert_eq!(collection.name, name);
        assert_eq!(collection.model, model.0);
        assert_eq!(collection.embedder, embedder.id());
        assert_eq!(collection.provider, vector_db.id());

        let v_collection = vector_db.get_collection(name).await.unwrap();

        let size = embedder.size(&collection.model).await.unwrap().unwrap();

        assert_eq!(size, v_collection.size);
    }

    #[test]
    async fn create_collection_fails_with_invalid_model(
        service: VectorService,
        vector_db: VectorDatabase,
        embedder: FastEmbedder,
    ) {
        let name = "test_collection_0";

        let params = CreateCollection {
            model: "invalid_model".to_string(),
            name: name.to_string(),
        };

        let result = service.create_collection(vector_db, embedder, params).await;

        assert!(result.is_err());
    }

    #[test]
    async fn create_collection_fails_with_existing_collection(
        service: VectorService,
        vector_db: VectorDatabase,
        embedder: FastEmbedder,
    ) {
        let params = CreateCollection {
            model: embedder.default_model().0,
            name: DEFAULT_COLLECTION_NAME.to_string(),
        };

        let result = service.create_collection(vector_db, embedder, params).await;

        assert!(result.is_err());
    }

    #[test]
    async fn inserting_and_searching_embeddings_works(
        service: VectorService,
        postgres: PgPool,
        vector_db: VectorDatabase,
        embedder: FastEmbedder,
    ) {
        let default = service
            .get_collection_by_name(DEFAULT_COLLECTION_NAME, vector_db.id())
            .await
            .unwrap();

        let create = DocumentInsert::new(
            "test_document",
            "test_path_1",
            DocumentType::Text,
            "SHA256_1",
            "fs",
        );

        let document = postgres.insert(create).await.unwrap();

        let content = r#"Hello World!"#;

        let embeddings = CreateEmbeddings {
            id: document.id,
            collection: default.id,
            chunks: &[content],
        };

        let collection = service
            .get_collection_by_name(DEFAULT_COLLECTION_NAME, vector_db.id())
            .await
            .unwrap();

        service
            .create_embeddings(vector_db, embedder, embeddings)
            .await
            .unwrap();

        let search = Search {
            query: content.to_string(),
            collection,
            limit: Some(1),
        };

        let results = service.search(vector_db, embedder, search).await.unwrap();

        assert_eq!(1, results.len());
        assert_eq!(content, results[0]);

        let embeddings = postgres
            .get_embeddings_by_name(document.id, DEFAULT_COLLECTION_NAME, vector_db.id())
            .await
            .unwrap()
            .unwrap();

        let collection = postgres
            .get_collection(embeddings.collection_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(DEFAULT_COLLECTION_NAME, collection.name);
        assert_eq!(document.id, embeddings.document_id);
    }

    #[test]
    async fn deleting_collection_removes_all_embeddings(
        service: VectorService,
        postgres: PgPool,
        vector_db: VectorDatabase,
        embedder: FastEmbedder,
    ) {
        let collection_name = "test_collection_delete_embeddings";

        let create = CreateCollection {
            name: collection_name.to_string(),
            model: embedder.default_model().0,
        };

        let collection = service
            .create_collection(vector_db, embedder, create)
            .await
            .unwrap();

        let create = DocumentInsert::new(
            "test_document",
            "test_path_2",
            DocumentType::Text,
            "SHA256_2",
            "fs",
        );

        let document = postgres.insert(create).await.unwrap();

        let content = r#"Hello World!"#;

        let embeddings = CreateEmbeddings {
            id: document.id,
            collection: collection.id,
            chunks: &[content],
        };

        service
            .create_embeddings(vector_db, embedder, embeddings)
            .await
            .unwrap();

        service
            .delete_collection(vector_db, collection.id)
            .await
            .unwrap();

        let embeddings = postgres
            .get_embeddings(document.id, collection.id)
            .await
            .unwrap();

        assert!(embeddings.is_none())
    }

    #[test]
    async fn prevents_duplicate_embeddings(
        service: VectorService,
        postgres: PgPool,
        vector_db: VectorDatabase,
        embedder: FastEmbedder,
    ) {
        let create = DocumentInsert::new(
            "test_document",
            "test_path_3",
            DocumentType::Text,
            "SHA256_3",
            "fs",
        );

        let default = service
            .get_collection_by_name(DEFAULT_COLLECTION_NAME, vector_db.id())
            .await
            .unwrap();

        let document = postgres.insert(create).await.unwrap();

        let content = r#"Hello World!"#;
        let create = CreateEmbeddings {
            id: document.id,
            collection: default.id,
            chunks: &[content],
        };
        service
            .create_embeddings(vector_db, embedder, create.clone())
            .await
            .unwrap();

        let duplicate = service.create_embeddings(vector_db, embedder, create).await;

        assert!(matches!(duplicate, Err(ChonkitError::AlreadyExists(_))))
    }
}
