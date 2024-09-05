use crate::{
    core::{
        model::collection::{Collection, CollectionInsert},
        model::{List, Pagination},
        repo::vector::VectorRepo,
    },
    error::ChonkitError,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

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
    async fn insert_collection(
        &self,
        collection: CollectionInsert<'_>,
    ) -> Result<Collection, ChonkitError> {
        let CollectionInsert {
            id,
            name,
            size,
            model,
        } = collection;
        let size = size as i32;
        let col = sqlx::query_as!(
            CollectionSelect,
            "INSERT INTO collections
                (id, name, model, size)
             VALUES
                ($1, $2, $3, $4)
             ON CONFLICT(id) DO UPDATE
             SET id = $1
             RETURNING 
                id, name, model, size, created_at, updated_at
             ",
            id,
            name,
            model,
            size
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Collection::from(col))
    }

    async fn get_collection(&self, id: Uuid) -> Result<Option<Collection>, ChonkitError> {
        Ok(sqlx::query_as!(
            CollectionSelect,
            "SELECT id, name, model, size, created_at, updated_at FROM collections WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .map(Collection::from))
    }

    async fn get_collection_by_name(&self, name: &str) -> Result<Option<Collection>, ChonkitError> {
        Ok(sqlx::query_as!(
            CollectionSelect,
            "SELECT id, name, model, size, created_at, updated_at FROM collections WHERE name = $1",
            name
        )
        .fetch_optional(&self.pool)
        .await?
        .map(Collection::from))
    }

    async fn delete_collection(&self, id: Uuid) -> Result<u64, ChonkitError> {
        let result = sqlx::query!("DELETE FROM collections WHERE id = $1", id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn list(&self, p: Pagination) -> Result<List<Collection>, ChonkitError> {
        let total = sqlx::query!("SELECT COUNT(id) FROM collections")
            .fetch_one(&self.pool)
            .await
            .map(|row| row.count.map(|count| count as usize))?;

        let (limit, offset) = p.to_limit_offset();
        let collections = sqlx::query_as!(
            CollectionSelect,
            r#"SELECT id, name, model, size, created_at, updated_at
                   FROM collections
                   LIMIT $1
                   OFFSET $2
                "#,
            limit,
            offset,
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Collection::from)
        .collect();

        Ok(List::new(total, collections))
    }

    async fn update_model(&self, id: Uuid, model: &str) -> Result<(), ChonkitError> {
        sqlx::query!("UPDATE collections SET model = $1 WHERE id = $2", model, id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

struct CollectionSelect {
    pub id: Uuid,
    pub name: String,
    pub model: Option<String>,
    pub size: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CollectionSelect> for Collection {
    fn from(
        CollectionSelect {
            id,
            name,
            model,
            size,
            created_at,
            updated_at,
        }: CollectionSelect,
    ) -> Self {
        Self {
            id,
            name,
            model,
            size: size as usize,
            created_at,
            updated_at,
        }
    }
}
