use crate::{
    core::{
        model::collection::{Collection, CollectionInsert},
        repo::{vector::VectorRepo, List},
    },
    error::ChonkitError,
};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PgVectorRepo {
    pool: sqlx::PgPool,
}

impl PgVectorRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl VectorRepo for PgVectorRepo {
    async fn create_collection(
        &self,
        collection: CollectionInsert<'_>,
    ) -> Result<Collection, ChonkitError> {
        let CollectionInsert { id, name, model } = collection;
        Ok(sqlx::query_as!(
            Collection,
            "INSERT INTO collections
                (id, name, model)
             VALUES
                ($1, $2, $3)
             RETURNING 
                id, name, model, created_at, updated_at",
            id,
            name,
            model
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_collection(&self, id: uuid::Uuid) -> Result<Option<Collection>, ChonkitError> {
        Ok(sqlx::query_as!(
            Collection,
            "SELECT id, name, model, created_at, updated_at FROM collections WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete_collection(&self, id: uuid::Uuid) -> Result<u64, ChonkitError> {
        let result = sqlx::query!("DELETE FROM collections WHERE id = $1", id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn list(
        &self,
        p: crate::core::repo::Pagination,
    ) -> Result<crate::core::repo::List<Collection>, ChonkitError> {
        let total = sqlx::query!("SELECT COUNT(id) FROM collections")
            .fetch_one(&self.pool)
            .await
            .map(|row| row.count.map(|count| count as usize))?;

        let collections = sqlx::query_as!(
            Collection,
            r#"SELECT id, name, model, created_at, updated_at
                   FROM collections
                   LIMIT $1
                   OFFSET $2
                "#,
            p.per_page as i64,
            p.page as i64,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(List::new(total, collections))
    }
}
