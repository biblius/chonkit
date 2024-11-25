use crate::{core::repo::Atomic, error::ChonkitError, map_err};
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
        let tx = map_err!(self.begin().await);
        Ok(tx)
    }

    async fn commit_tx(&self, tx: Self::Tx) -> Result<(), ChonkitError> {
        map_err!(tx.commit().await);
        Ok(())
    }

    async fn abort_tx(&self, tx: Self::Tx) -> Result<(), ChonkitError> {
        map_err!(tx.rollback().await);
        Ok(())
    }
}
