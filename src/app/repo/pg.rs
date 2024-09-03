use sqlx::PgPool;
use tracing::info;

pub mod document;
pub mod vector;

pub async fn init(url: &str) -> PgPool {
    info!("Connecting to postgres at {url}");
    let pool = create_pool(url).await;
    migrate(&pool).await;
    pool
}

async fn create_pool(url: &str) -> PgPool {
    sqlx::postgres::PgPool::connect(url)
        .await
        .expect("error while connecting to db")
}

async fn migrate(pool: &PgPool) {
    sqlx::migrate!()
        .run(pool)
        .await
        .expect("error in migrations")
}
