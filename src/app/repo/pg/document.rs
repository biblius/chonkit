use crate::core::{
    chunk::ChunkConfig,
    document::parser::Parser,
    model::document::{
        config::{
            DocumentChunkConfig, DocumentChunkConfigInsert, DocumentParseConfig,
            DocumentParseConfigInsert,
        },
        Document, DocumentInsert, DocumentUpdate,
    },
    repo::{document::DocumentRepo, List},
};
use crate::error::ChonkitError;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{types::Json, PgPool};

#[derive(Debug, Clone)]
pub struct PgDocumentRepo {
    pool: sqlx::PgPool,
}

impl PgDocumentRepo {
    pub fn new(pool: PgPool) -> Self {
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
            ext.to_string(),
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

    async fn get_chunk_config(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<DocumentChunkConfig>, ChonkitError> {
        Ok(sqlx::query_as!(
            SelectConfig::<ChunkConfig>,
            r#"SELECT 
                id,
                document_id,
                config AS "config: _",
                created_at,
                updated_at 
             FROM chunkers 
             WHERE document_id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .map(DocumentChunkConfig::from))
    }

    async fn get_parse_config(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<DocumentParseConfig>, ChonkitError> {
        Ok(sqlx::query_as!(
            SelectConfig::<Parser>,
            r#"SELECT 
                id,
                document_id,
                config AS "config: _",
                created_at,
                updated_at 
             FROM parsers 
             WHERE document_id = $1"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?
        .map(DocumentParseConfig::from))
    }

    async fn insert_chunk_config(
        &self,
        config: DocumentChunkConfigInsert,
    ) -> Result<DocumentChunkConfig, ChonkitError> {
        let config = InsertConfig::from(config);

        let InsertConfig {
            id,
            document_id,
            config,
        } = config;

        let config = sqlx::query_as!(
            SelectConfig::<ChunkConfig>,
            r#"INSERT INTO chunkers
                (id, document_id, config)
             VALUES
                ($1, $2, $3)
             RETURNING
                id, document_id, config AS "config: _", created_at, updated_at
            "#,
            id,
            document_id,
            config as Json<ChunkConfig>,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DocumentChunkConfig::from(config))
    }

    async fn insert_parse_config(
        &self,
        config: DocumentParseConfigInsert,
    ) -> Result<DocumentParseConfig, ChonkitError> {
        let config = InsertConfig::from(config);

        let InsertConfig {
            id,
            document_id,
            config,
        } = config;

        let config = sqlx::query_as!(
            SelectConfig::<Parser>,
            r#"INSERT INTO parsers
                (id, document_id, config)
             VALUES
                ($1, $2, $3)
             RETURNING
                id, document_id, config AS "config: _", created_at, updated_at"#,
            id,
            document_id,
            config as Json<Parser>,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(DocumentParseConfig::from(config))
    }
}

// Private dtos.

struct InsertConfig<T: Serialize> {
    pub id: uuid::Uuid,
    pub document_id: uuid::Uuid,
    pub config: sqlx::types::Json<T>,
}

impl From<DocumentParseConfigInsert> for InsertConfig<Parser> {
    fn from(value: DocumentParseConfigInsert) -> Self {
        Self {
            id: value.id,
            document_id: value.document_id,
            config: sqlx::types::Json(value.config),
        }
    }
}

impl From<DocumentChunkConfigInsert> for InsertConfig<ChunkConfig> {
    fn from(value: DocumentChunkConfigInsert) -> Self {
        Self {
            id: value.id,
            document_id: value.document_id,
            config: sqlx::types::Json(value.config),
        }
    }
}

struct SelectConfig<T: DeserializeOwned> {
    id: uuid::Uuid,
    document_id: uuid::Uuid,
    config: sqlx::types::Json<T>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SelectConfig<ChunkConfig>> for DocumentChunkConfig {
    fn from(value: SelectConfig<ChunkConfig>) -> Self {
        let SelectConfig {
            id,
            document_id,
            config,
            created_at,
            updated_at,
        } = value;
        Self {
            id,
            document_id,
            config: config.0,
            created_at,
            updated_at,
        }
    }
}

impl From<SelectConfig<Parser>> for DocumentParseConfig {
    fn from(value: SelectConfig<Parser>) -> Self {
        let SelectConfig {
            id,
            document_id,
            config,
            created_at,
            updated_at,
        } = value;
        Self {
            id,
            document_id,
            config: config.0,
            created_at,
            updated_at,
        }
    }
}

#[cfg(test)]
#[suitest::suite(pg_document_repo_int)]
mod tests {

    use super::PgDocumentRepo;
    use crate::{
        app::repo::pg::init,
        core::{
            model::document::{DocumentInsert, DocumentType},
            repo::document::DocumentRepo,
        },
    };
    use suitest::before_all;

    #[before_all]
    async fn setup() -> PgDocumentRepo {
        let url = std::env::var("DATABASE_URL").expect("no database url");
        let client = init(&url).await;

        let repo = PgDocumentRepo::new(client);

        repo
    }

    #[test]
    async fn inserting_document_works(repo: PgDocumentRepo) {
        let doc = DocumentInsert::new("My file", "path/to/file", DocumentType::Text);
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
