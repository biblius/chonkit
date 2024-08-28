use crate::error::ChonkitError;
use crate::model::document::{File, FileInsert, FileUpdate};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct DocumentDb {
    pool: sqlx::PgPool,
}

impl DocumentDb {
    pub async fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_file(&self, id: uuid::Uuid) -> Result<Option<File>, ChonkitError> {
        Ok(
            sqlx::query_as!(File, "SELECT * FROM files WHERE id = $1", id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    /// Retrieve all paths from the documents table
    pub async fn get_all_file_paths(&self) -> Result<Vec<String>, ChonkitError> {
        Ok(sqlx::query!("SELECT path FROM files",)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|el| el.path)
            .collect())
    }

    pub async fn get_file_path(&self, id: uuid::Uuid) -> Result<Option<String>, ChonkitError> {
        Ok(sqlx::query!("SELECT path FROM files WHERE id = $1", id)
            .fetch_optional(&self.pool)
            .await?
            .map(|el| el.path))
    }

    pub async fn insert_file(&self, file: FileInsert<'_>) -> Result<File, ChonkitError> {
        let FileInsert {
            name,
            path,
            label,
            tags,
        } = file;

        sqlx::query_as!(File,
            "INSERT INTO files(name, path, label, tags) VALUES($1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING *",
            name,
            path,
            label,
            tags.as_deref(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(ChonkitError::from)
    }

    pub async fn get_file_by_path(&self, path: &str) -> Result<Option<File>, ChonkitError> {
        sqlx::query_as!(File, "SELECT * FROM files WHERE path = $1", path)
            .fetch_optional(&self.pool)
            .await
            .map_err(ChonkitError::from)
    }

    pub async fn update_file_by_path(
        &self,
        path: &str,
        update: &FileUpdate<'_>,
    ) -> Result<(), ChonkitError> {
        let FileUpdate {
            name,
            path: file_path,
            label,
            tags,
        } = update;

        let tags = tags.as_ref().map(|v| v.as_slice());

        sqlx::query!(
            r#"
            UPDATE files SET 
            name = $1,
            path = $2,
            label = $3,
            tags = $4
            WHERE path = $5 
        "#,
            name.as_ref(),
            file_path.as_ref(),
            label.as_ref(),
            tags.as_ref(),
            path
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn remove_file_by_path(&self, path: &str) -> Result<(), ChonkitError> {
        sqlx::query!("DELETE FROM files WHERE path = $1", path)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
