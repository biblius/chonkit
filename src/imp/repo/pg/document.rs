use crate::core::{
    model::document::{
        config::{DocumentConfig, DocumentConfigInsert},
        Document, DocumentInsert, DocumentUpdate,
    },
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
            r#"SELECT id, name, path, ext, label, tags, created_at, updated_at
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
            ext,
            label,
            tags,
        } = file;

        sqlx::query_as!(
            Document,
            "INSERT INTO documents VALUES($1, $2, $3, $4, $5, $6) ON CONFLICT DO NOTHING RETURNING *",
            id,
            name,
            path,
            ext,
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
            path,
            label,
            tags,
        } = update;

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
            path.as_ref(),
            label.as_ref(),
            tags.as_deref(),
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

    async fn get_config(&self, id: uuid::Uuid) -> Result<Option<DocumentConfig>, ChonkitError> {
        sqlx::query_as!(
            DocumentConfig,
            "SELECT 
                id,
                document_id,
                chunk_config,
                parse_config,
                created_at,
                updated_at 
             FROM doc_configs 
             WHERE document_id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        todo!()
    }

    async fn insert_config(
        &self,
        config: DocumentConfigInsert,
    ) -> Result<DocumentConfig, ChonkitError> {
        let DocumentConfigInsert {
            id,
            document_id,
            chunk_config,
            parse_config,
        } = config;

        let config = sqlx::query_as!(
            DocumentConfig,
            "INSERT INTO doc_configs
                (id, document_id, chunk_config, parse_config)
             VALUES
                ($1, $2, $3, $4)
             RETURNING
                id, document_id, chunk_config, parse_config, created_at, updated_at",
            id,
            document_id,
            chunk_config,
            parse_config
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(config)
    }
}

#[cfg(test)]
#[suitest::suite(pg_document_repo_int)]
mod tests {

    use super::PgDocumentRepo;
    use crate::{
        core::{model::document::DocumentInsert, repo::document::DocumentRepo},
        imp::repo::pg::init,
    };
    use suitest::before_all;

    #[before_all]
    async fn setup() -> PgDocumentRepo {
        let url = std::env::var("DATABASE_URL").expect("no database url");
        let client = init(&url).await;

        let repo = PgDocumentRepo::new(client).await;

        repo
    }

    #[test]
    async fn insert_works(repo: PgDocumentRepo) {
        let doc = DocumentInsert::new("My file", "path/to/file", "txt");
        let doc = repo.insert(doc).await.unwrap();
        let doc = repo.get_by_id(doc.id).await.unwrap().unwrap();

        assert_eq!("My file", doc.name);
        assert_eq!("path/to/file", doc.path);
        assert_eq!("txt", doc.ext);

        repo.remove_by_id(doc.id).await.unwrap();

        let doc = repo.get_by_id(doc.id).await.unwrap();

        assert!(doc.is_none());
    }
}
