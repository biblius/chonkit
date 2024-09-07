use crate::{
    app::{embedder::FastEmbedder, vector::qdrant::QdrantDb},
    core::service::vector::VectorService as Service,
};
use sqlx::PgPool;

pub(in crate::app) type VectorService = Service<PgPool, QdrantDb, FastEmbedder>;

#[cfg(test)]
#[suitest::suite(integration_tests)]
#[suitest::suite_cfg(verbose = true)]
mod vector_service_postgres_qdrant_fastembed {
    use super::VectorService;
    use crate::{
        app::{
            embedder::FastEmbedder,
            test::{init_postgres, init_qdrant, PostgresContainer},
            vector::qdrant::QdrantDb,
        },
        core::{
            embedder::Embedder, model::document::DocumentInsert, repo::document::DocumentRepo,
            service::vector::dto::CreateCollection, vector::VectorDb,
        },
        DEFAULT_COLLECTION_MODEL, DEFAULT_COLLECTION_NAME,
    };
    use sqlx::PgPool;
    use suitest::before_all;
    use testcontainers::{ContainerAsync, GenericImage};

    #[before_all]
    async fn setup() -> (
        PgPool,
        QdrantDb,
        VectorService,
        FastEmbedder,
        PostgresContainer,
        ContainerAsync<GenericImage>,
    ) {
        let (postgres, pg) = init_postgres().await;
        let (qdrant, qd) = init_qdrant().await;

        let embedder = FastEmbedder;

        let service = VectorService::new(postgres.clone(), qdrant.clone(), embedder.clone());

        (postgres, qdrant, service, embedder, pg, qd)
    }

    #[test]
    async fn default_collection_is_stored_in_repo(
        service: VectorService,
        embedder: FastEmbedder,
        qdrant: QdrantDb,
    ) {
        service.create_default_collection().await;

        let collection = service
            .get_collection(DEFAULT_COLLECTION_NAME)
            .await
            .unwrap();

        assert_eq!(collection.name, DEFAULT_COLLECTION_NAME);
        assert_eq!(collection.model, DEFAULT_COLLECTION_MODEL);
        assert_eq!(collection.embedder, embedder.id());
        assert_eq!(collection.src, qdrant.id());
    }

    #[test]
    async fn default_collection_is_stored_vec_db(
        service: VectorService,
        embedder: FastEmbedder,
        qdrant: QdrantDb,
    ) {
        service.create_default_collection().await;

        let collection = service
            .get_collection(DEFAULT_COLLECTION_NAME)
            .await
            .unwrap();

        let v_collection = qdrant
            .get_collection(DEFAULT_COLLECTION_NAME)
            .await
            .unwrap();

        let size = embedder.size(&collection.model).unwrap();

        assert_eq!(size, v_collection.size);

        // Assert this can be called again without errors.
        service.create_default_collection().await;
    }

    #[test]
    async fn create_collection_works(
        service: VectorService,
        embedder: FastEmbedder,
        qdrant: QdrantDb,
    ) {
        let model = embedder.list_embedding_models().first().cloned().unwrap();

        let params = CreateCollection {
            model: model.0.clone(),
            name: "__test_collection__".to_string(),
        };

        let collection = service.create_collection(params).await.unwrap();

        assert_eq!(collection.name, "__test_collection__");
        assert_eq!(collection.model, model.0);
        assert_eq!(collection.embedder, embedder.id());
        assert_eq!(collection.src, qdrant.id());

        let v_collection = qdrant.get_collection("__test_collection__").await.unwrap();

        let size = embedder.size(&collection.model).unwrap();

        assert_eq!(size, v_collection.size);
    }

    #[test]
    async fn create_collection_fails_with_invalid_model(service: VectorService) {
        let params = CreateCollection {
            model: "invalid_model".to_string(),
            name: "__test_collection__".to_string(),
        };

        let result = service.create_collection(params).await;

        assert!(result.is_err());
    }

    #[test]
    async fn create_collection_fails_with_existing_collection(service: VectorService) {
        let params = CreateCollection {
            model: DEFAULT_COLLECTION_MODEL.to_string(),
            name: DEFAULT_COLLECTION_NAME.to_string(),
        };

        let result = service.create_collection(params).await;

        assert!(result.is_err());
    }

    // #[test]
    // async fn inserting_embeddings_works(service: VectorService, postgres: PgPool) {
    //     let create = DocumentInsert::new("test_document", "test_path");
    //     let document = postgres.insert(document).await.unwrap();
    //
    //     let content = "Hello World!";
    //     service
    //         .create_embeddings(id, collection, content)
    //         .await
    //         .unwrap();
    // }
}
