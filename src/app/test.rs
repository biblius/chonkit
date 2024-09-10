//! Test container utilites.

use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage,
};
use testcontainers_modules::postgres::Postgres;

#[cfg(feature = "qdrant")]
use super::vector::qdrant::QdrantDb;

#[cfg(feature = "weaviate")]
use super::vector::weaviate::WeaviateDb;

pub type PostgresContainer = ContainerAsync<Postgres>;
pub type AsyncContainer = ContainerAsync<GenericImage>;

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
pub async fn init_qdrant() -> (QdrantDb, ContainerAsync<GenericImage>) {
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
pub async fn init_weaviate() -> (WeaviateDb, ContainerAsync<GenericImage>) {
    use testcontainers::ImageExt;

    let wv_image = GenericImage::new("semitechnologies/weaviate", "latest")
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
