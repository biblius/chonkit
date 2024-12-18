//! Test suites and utilites.

mod document;
mod vector;

use super::{
    document::store::FsDocumentStore,
    state::{
        AppProviderState, AppState, DocumentStoreProvider, EmbeddingProvider, ServiceState,
        VectorDbProvider,
    },
};
use crate::core::service::{document::DocumentService, vector::VectorService};
use crate::{
    config::DEFAULT_COLLECTION_EMBEDDING_MODEL,
    core::{
        document::store::DocumentStore, embedder::Embedder, provider::ProviderFactory,
        vector::VectorDb,
    },
};
use std::sync::Arc;
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage};
use testcontainers_modules::postgres::Postgres;

pub type PostgresContainer = ContainerAsync<Postgres>;
pub type AsyncContainer = ContainerAsync<GenericImage>;

struct TestState {
    /// Holds test containers so they don't get dropped.
    pub _containers: TestContainers,

    /// Holds the downstream service providers necessary for chonkit services.
    pub app: AppState,

    /// Holds the list of active vector storage providers. Depends on feature flags.
    pub active_vector_providers: Vec<&'static str>,

    /// Holds the list of active embedding providers. Depends on feature flags.
    pub active_embedding_providers: Vec<&'static str>,
}

impl TestState {
    pub async fn init(config: TestStateConfig) -> Self {
        // Set up test containers

        let (postgres, postgres_img) = init_postgres().await;

        #[cfg(feature = "qdrant")]
        let (qdrant, qdrant_img) = init_qdrant().await;

        #[cfg(feature = "weaviate")]
        let (weaviate, weaviate_img) = init_weaviate().await;

        // Set up document storage

        let mut store = DocumentStoreProvider::default();

        let fs_store = Arc::new(FsDocumentStore::new(&config.fs_store_path));
        store.register(fs_store.id(), fs_store);

        // Set up vector storage

        let mut vector = VectorDbProvider::default();
        let mut active_vector_providers = vec![];

        #[cfg(feature = "qdrant")]
        {
            active_vector_providers.push(qdrant.id());
            vector.register(qdrant.id(), qdrant);
        }

        #[cfg(feature = "weaviate")]
        {
            active_vector_providers.push(weaviate.id());
            vector.register(weaviate.id(), weaviate);
        }

        // Set up embedders

        let mut embedding = EmbeddingProvider::default();
        let mut active_embedding_providers = vec![];

        #[cfg(feature = "fe-local")]
        {
            let fastembed = Arc::new(
                crate::app::embedder::fastembed::local::LocalFastEmbedder::new_with_model(
                    DEFAULT_COLLECTION_EMBEDDING_MODEL,
                ),
            );
            active_embedding_providers.push(fastembed.id());
            embedding.register(fastembed.id(), fastembed);
        }

        // If active, overrides the fe-local implementation since we keep it on the same ID.
        #[cfg(feature = "fe-remote")]
        {
            let fastembed = Arc::new(
                crate::app::embedder::fastembed::remote::RemoteFastEmbedder::new(
                    String::new(), /* TODO */
                ),
            );
            if !active_embedding_providers.contains(&fastembed.id()) {
                active_embedding_providers.push(fastembed.id());
            }
            embedding.register(fastembed.id(), fastembed);
        }

        let providers = AppProviderState {
            database: postgres.clone(),
            vector: Arc::new(vector.clone()),
            embedding: Arc::new(embedding),
            document: Arc::new(store.clone()),
        };

        let _containers = TestContainers {
            _postgres: postgres_img,
            #[cfg(feature = "qdrant")]
            _qdrant: qdrant_img,
            #[cfg(feature = "weaviate")]
            _weaviate: weaviate_img,
        };

        let services = ServiceState {
            vector: VectorService::new(postgres.clone(), providers.clone().into()),
            document: DocumentService::new(postgres, providers.clone().into()),
        };

        let app = AppState::new_test(
            services,
            providers,
            #[cfg(feature = "auth-vault")]
            todo!(),
        );

        TestState {
            _containers,
            app,
            active_vector_providers,
            active_embedding_providers,
        }
    }
}

struct TestStateConfig {
    pub fs_store_path: String,
}

/// Holds test container images so they don't get dropped during execution of test suites.
struct TestContainers {
    pub _postgres: PostgresContainer,

    #[cfg(feature = "qdrant")]
    pub _qdrant: ContainerAsync<GenericImage>,

    #[cfg(feature = "weaviate")]
    pub _weaviate: ContainerAsync<GenericImage>,
}

/// Setup a postgres test container and connect to it using PgPool.
/// Runs the migrations in the container.
/// When using suitest's [before_all][suitest::before_all], make sure you return this, othwerise the
/// container will get dropped and cleaned up.
pub async fn init_postgres() -> (sqlx::PgPool, PostgresContainer) {
    let pg_image = Postgres::default()
        .start()
        .await
        .expect("postgres container error");

    let pg_host = pg_image.get_host().await.unwrap();
    let pg_port = pg_image.get_host_port_ipv4(5432).await.unwrap();
    let pg_url = format!("postgresql://postgres:postgres@{pg_host}:{pg_port}/postgres");
    (crate::app::repo::pg::init(&pg_url).await, pg_image)
}

/// Setup a qdrant test container and connect to it using QdrantDb.
/// When using suitest's [before_all][suitest::before_all], make sure you return this, othwerise the
/// container will get dropped and cleaned up.
#[cfg(feature = "qdrant")]
pub async fn init_qdrant() -> (
    super::vector::qdrant::QdrantDb,
    ContainerAsync<GenericImage>,
) {
    use testcontainers::core::{IntoContainerPort, WaitFor};

    let qd_image = GenericImage::new("qdrant/qdrant", "latest")
        .with_exposed_port(6334.tcp())
        .with_wait_for(WaitFor::message_on_stdout("gRPC listening on"))
        .start()
        .await
        .expect("qdrant container error");

    let qd_host = qd_image.get_host().await.unwrap();
    let qd_port = qd_image.get_host_port_ipv4(6334).await.unwrap();
    let qd_url = format!("http://{qd_host}:{qd_port}");
    (crate::app::vector::qdrant::init(&qd_url), qd_image)
}

/// Setup a weaviate test container and connect to it using WeaviateDb.
/// When using suitest's [before_all][suitest::before_all], make sure you return this, othwerise the
/// container will get dropped and cleaned up.
#[cfg(feature = "weaviate")]
pub async fn init_weaviate() -> (
    super::vector::weaviate::WeaviateDb,
    ContainerAsync<GenericImage>,
) {
    use testcontainers::core::{ImageExt, IntoContainerPort, WaitFor};

    let wv_image = GenericImage::new("semitechnologies/weaviate", "1.24.12")
        .with_exposed_port(8080.tcp())
        .with_exposed_port(50051.tcp())
        .with_wait_for(WaitFor::message_on_stderr("Serving weaviate"))
        .with_env_var("AUTHENTICATION_ANONYMOUS_ACCESS_ENABLED", "true")
        .with_env_var("PERSISTENCE_DATA_PATH", "/var/lib/weaviate")
        .start()
        .await
        .expect("weaviate container error");

    let wv_host = wv_image.get_host().await.unwrap();
    let wv_port = wv_image.get_host_port_ipv4(8080).await.unwrap();
    let wv_url = format!("http://{wv_host}:{wv_port}");
    (crate::app::vector::weaviate::init(&wv_url), wv_image)
}
