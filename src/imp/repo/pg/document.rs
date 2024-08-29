use crate::core::{
    model::document::{Document, DocumentInsert, DocumentUpdate},
    repo::{document::DocumentRepo, List},
};
use crate::error::ChonkitError;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct PgDocumentRepo {
    pool: sqlx::PgPool,
}

impl PgDocumentRepo {
    pub async fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl DocumentRepo for PgDocumentRepo {
    async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<Document>, ChonkitError> {
        Ok(
            sqlx::query_as!(Document, "SELECT * FROM documents WHERE id = $1", id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    async fn get_by_path(&self, path: &str) -> Result<Option<Document>, ChonkitError> {
        sqlx::query_as!(Document, "SELECT * FROM documents WHERE path = $1", path)
            .fetch_optional(&self.pool)
            .await
            .map_err(ChonkitError::from)
    }

    async fn get_path(&self, id: uuid::Uuid) -> Result<Option<String>, ChonkitError> {
        Ok(sqlx::query!("SELECT path FROM documents WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await?
            .map(|el| el.path))
    }

    async fn list(&self, p: crate::core::repo::Pagination) -> Result<List<Document>, ChonkitError> {
        let total = sqlx::query!("SELECT COUNT(id) FROM documents")
            .fetch_one(&self.pool)
            .await
            .map(|row| row.count)?;

        let documents = sqlx::query_as!(
            Document,
            r#"SELECT id, name, path, label, tags, created_at, updated_at
                   FROM documents
                   LIMIT $1
                   OFFSET $2
                "#,
            p.per_page as i64,
            p.page as i64,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(List::new(total.map(|c| c as usize), documents))
    }

    async fn insert(&self, file: DocumentInsert<'_>) -> Result<Document, ChonkitError> {
        let DocumentInsert {
            id,
            name,
            path,
            label,
            tags,
        } = file;

        sqlx::query_as!(Document,
            "INSERT INTO documents(id, name, path, label, tags) VALUES($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING RETURNING *",
            id,
            name,
            path,
            label,
            tags.as_deref(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(ChonkitError::from)
    }

    async fn update(&self, id: uuid::Uuid, update: DocumentUpdate<'_>) -> Result<(), ChonkitError> {
        let DocumentUpdate {
            name,
            path: file_path,
            label,
            tags,
        } = update;

        let tags = tags.as_ref().map(|v| v.as_slice());

        sqlx::query!(
            r#"
            UPDATE documents SET 
            name = $1,
            path = $2,
            label = $3,
            tags = $4
            WHERE id = $5 
        "#,
            name.as_ref(),
            file_path.as_ref(),
            label.as_ref(),
            tags.as_ref(),
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_by_id(&self, id: uuid::Uuid) -> Result<(), ChonkitError> {
        sqlx::query!("DELETE FROM documents WHERE id = $1", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_by_path(&self, path: &str) -> Result<(), ChonkitError> {
        sqlx::query!("DELETE FROM documents WHERE path = $1", path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
