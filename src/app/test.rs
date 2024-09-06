//! Test container utilites.

use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, GenericImage,
};
use testcontainers_modules::postgres::Postgres;

pub type PostgresContainer = ContainerAsync<Postgres>;

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

/// Setup a qdrant test container and connect to it using Qdrant.
/// When using suitest's [before_all][suitest::before_all], make sure you return this, othwerise the
/// container will get dropped and cleaned up.
pub async fn init_qdrant() -> (qdrant_client::Qdrant, ContainerAsync<GenericImage>) {
    let qd_image = GenericImage::new("qdrant/qdrant", "latest")
        .with_exposed_port(6334.tcp())
        .with_wait_for(WaitFor::message_on_stdout("gRPC listening on"))
        .start()
        .await
        .expect("qdrant container error");

    let qd_host = qd_image.get_host().await.unwrap();
    let qd_port = qd_image.get_host_port_ipv4(6334).await.unwrap();
    let qd_url = format!("http://{qd_host}:{qd_port}");
    (crate::app::vector::store::qdrant::init(&qd_url), qd_image)
}
