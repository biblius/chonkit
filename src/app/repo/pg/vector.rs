use crate::{
    core::{
        model::{
            collection::{Collection, CollectionInsert, Embedding, EmbeddingInsert},
            List, Pagination,
        },
        repo::{vector::VectorRepo, Atomic},
    },
    error::ChonkitError,
};
use sqlx::PgPool;
use uuid::Uuid;

impl VectorRepo<<PgPool as Atomic>::Tx> for PgPool {
    async fn list_collections(&self, p: Pagination) -> Result<List<Collection>, ChonkitError> {
        let total = sqlx::query!("SELECT COUNT(name) FROM collections")
            .fetch_one(self)
            .await
            .map(|row| row.count.map(|count| count as usize))?;

        let (limit, offset) = p.to_limit_offset();
        let collections = sqlx::query_as!(
            Collection,
            r#"SELECT name, model, embedder, src, created_at, updated_at
                   FROM collections
                   LIMIT $1
                   OFFSET $2
                "#,
            limit,
            offset,
        )
        .fetch_all(self)
        .await?
        .into_iter()
        .collect();

        Ok(List::new(total, collections))
    }

    async fn insert_collection(
        &self,
        insert: CollectionInsert<'_>,
        tx: Option<&mut <PgPool as Atomic>::Tx>,
    ) -> Result<Collection, ChonkitError> {
        let CollectionInsert {
            name,
            model,
            embedder,
            src,
        } = insert;

        let query = sqlx::query_as!(
            Collection,
            "INSERT INTO collections
                (name, model, embedder, src)
             VALUES
                ($1, $2, $3, $4)
             RETURNING 
                name, model, embedder, src, created_at, updated_at
             ",
            name,
            model,
            embedder,
            src
        );

        let collection = if let Some(tx) = tx {
            query.fetch_one(&mut **tx).await
        } else {
            query.fetch_one(self).await
        };

        collection.map_err(|e| match e {
            sqlx::Error::Database(err) if err.code().is_some_and(|code| code == "23505") => {
                ChonkitError::AlreadyExists(format!("Collection '{name}' already exists"))
            }
            _ => ChonkitError::from(e),
        })
    }

    async fn delete_collection(&self, name: &str) -> Result<u64, ChonkitError> {
        let result = sqlx::query!("DELETE FROM collections WHERE name = $1", name)
            .execute(self)
            .await?;
        Ok(result.rows_affected())
    }

    async fn get_collection(&self, name: &str) -> Result<Option<Collection>, ChonkitError> {
        Ok(sqlx::query_as!(
            Collection,
            "SELECT name, model, embedder, src, created_at, updated_at FROM collections WHERE name = $1",
            name
        )
        .fetch_optional(self)
        .await?)
    }

    async fn insert_embeddings(
        &self,
        embeddings: EmbeddingInsert<'_>,
    ) -> Result<Embedding, ChonkitError> {
        let EmbeddingInsert {
            id,
            document_id,
            collection,
        } = embeddings;

        Ok(sqlx::query_as!(
            Embedding,
            "INSERT INTO embeddings
                (id, document_id, collection)
             VALUES
                ($1, $2, $3)
             ON CONFLICT(id) DO UPDATE
             SET id = $1
             RETURNING 
                id, document_id, collection, created_at, updated_at
             ",
            id,
            document_id,
            collection,
        )
        .fetch_one(self)
        .await?)
    }

    async fn get_all_embeddings(&self, id: Uuid) -> Result<Vec<Embedding>, ChonkitError> {
        Ok(sqlx::query_as!(
            Embedding,
            "SELECT id, document_id, collection, created_at, updated_at 
             FROM embeddings
             WHERE document_id = $1",
            id
        )
        .fetch_all(self)
        .await?)
    }

    async fn get_embeddings(
        &self,
        id: Uuid,
        collection: &str,
    ) -> Result<Option<Embedding>, ChonkitError> {
        Ok(sqlx::query_as!(
            Embedding,
            "SELECT id, document_id, collection, created_at, updated_at 
             FROM embeddings
             WHERE document_id = $1 AND collection = $2",
            id,
            collection
        )
        .fetch_optional(self)
        .await?)
    }

    async fn delete_embeddings(&self, id: Uuid, collection: &str) -> Result<u64, ChonkitError> {
        Ok(sqlx::query!(
            "DELETE FROM embeddings WHERE document_id = $1 AND collection = $2",
            id,
            collection
        )
        .execute(self)
        .await?
        .rows_affected())
    }

    async fn delete_all_embeddings(&self, collection: &str) -> Result<u64, ChonkitError> {
        Ok(
            sqlx::query!("DELETE FROM embeddings WHERE collection = $1", collection)
                .execute(self)
                .await?
                .rows_affected(),
        )
    }
}
