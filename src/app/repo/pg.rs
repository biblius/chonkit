use sqlx::PgPool;
use tracing::info;

pub mod document;
pub mod vector;

pub async fn init(url: &str) -> PgPool {
    info!("Connecting to postgres at {url}");

    let pool = sqlx::postgres::PgPool::connect(url)
        .await
        .expect("error while connecting to db");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("error in migrations");

    pool
}
