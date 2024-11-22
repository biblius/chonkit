use crate::{core::repo::Atomic, error::ChonkitError};
use sqlx::{PgPool, Transaction};
use tracing::info;

pub mod document;
pub mod vector;

pub async fn init(url: &str) -> PgPool {
    let pool = sqlx::postgres::PgPool::connect(url)
        .await
        .expect("error while connecting to db");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("error in migrations");

    info!("Connected to postgres");
    pool
}

impl Atomic for PgPool {
    type Tx = Transaction<'static, sqlx::Postgres>;

    async fn start_tx(&self) -> Result<Self::Tx, ChonkitError> {
        self.begin().await.map_err(ChonkitError::from)
    }

    async fn commit_tx(&self, tx: Self::Tx) -> Result<(), ChonkitError> {
        tx.commit().await.map_err(ChonkitError::from)
    }

    async fn abort_tx(&self, tx: Self::Tx) -> Result<(), ChonkitError> {
        tx.rollback().await.map_err(ChonkitError::from)
    }
}
