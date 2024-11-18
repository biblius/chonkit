//! Test suites and utilites.

mod document;
mod vector;

use super::{
    document::store::FsDocumentStore,
    state::{DocumentStoreProvider, EmbeddingProvider, VectorStoreProvider},
};
use crate::core::provider::ProviderState;
use std::sync::Arc;
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage};
use testcontainers_modules::postgres::Postgres;

pub type PostgresContainer = ContainerAsync<Postgres>;
pub type AsyncContainer = ContainerAsync<GenericImage>;

struct TestState {
    /// Individual clients for repository and vector database implementations.
    pub clients: TestClients,

    /// Holds test containers so they don't get dropped.
    pub containers: TestContainers,

    /// Holds the providers necessary for services.
    pub providers: ProviderState,
}

impl TestState {
    pub async fn init(config: TestStateConfig) -> Self {
        let (postgres, postgres_img) = init_postgres().await;

        #[cfg(feature = "qdrant")]
        let (qdrant, qdrant_img) = init_qdrant().await;

        #[cfg(feature = "weaviate")]
        let (weaviate, weaviate_img) = init_weaviate().await;

        #[cfg(feature = "fembed")]
        let fastembed = Arc::new(crate::app::embedder::fastembed::FastEmbedder::new());

        let vector = VectorStoreProvider {
            #[cfg(feature = "qdrant")]
            qdrant: qdrant.clone(),

            #[cfg(feature = "weaviate")]
            weaviate: weaviate.clone(),
        };

        let store = DocumentStoreProvider {
            fs_store: Arc::new(FsDocumentStore::new(&config.fs_store_path)),
        };

        let embedding = EmbeddingProvider {
            #[cfg(feature = "fembed")]
            fastembed: fastembed.clone(),

            #[cfg(feature = "openai")]
            openai: panic!("cannot run test with `openai` enabled"),
        };

        let providers = ProviderState {
            vector: Arc::new(vector.clone()),
            embedding: Arc::new(embedding.clone()),
            store: Arc::new(store.clone()),
        };

        let clients = TestClients {
            postgres,
            #[cfg(feature = "qdrant")]
            qdrant,
            #[cfg(feature = "weaviate")]
            weaviate,
            #[cfg(feature = "fembed")]
            fastembed,
        };

        let containers = TestContainers {
            postgres: postgres_img,
            #[cfg(feature = "qdrant")]
            qdrant: qdrant_img,
            #[cfg(feature = "weaviate")]
            weaviate: weaviate_img,
        };

        TestState {
            clients,
            containers,
            providers,
        }
    }
}

struct TestStateConfig {
    pub fs_store_path: String,
}

/// Holds clients for repository and vector database implementations.
struct TestClients {
    pub postgres: sqlx::PgPool,

    #[cfg(feature = "qdrant")]
    pub qdrant: super::vector::qdrant::QdrantDb,

    #[cfg(feature = "weaviate")]
    pub weaviate: super::vector::weaviate::WeaviateDb,

    #[cfg(feature = "fembed")]
    pub fastembed: std::sync::Arc<super::embedder::fastembed::FastEmbedder>,
}

/// Holds test container images so they don't get dropped during execution of test suites.
struct TestContainers {
    pub postgres: PostgresContainer,

    #[cfg(feature = "qdrant")]
    pub qdrant: ContainerAsync<GenericImage>,

    #[cfg(feature = "weaviate")]
    pub weaviate: ContainerAsync<GenericImage>,
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
