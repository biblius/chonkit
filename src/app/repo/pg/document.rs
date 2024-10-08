use crate::core::{
    chunk::Chunker,
    document::parser::ParseConfig,
    model::{
        document::{
            config::{DocumentChunkConfig, DocumentParseConfig},
            Document, DocumentConfig, DocumentInsert, DocumentUpdate,
        },
        List, Pagination,
    },
    repo::document::DocumentRepo,
};
use crate::error::ChonkitError;
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{types::Json, PgPool};

#[async_trait::async_trait]
impl DocumentRepo for PgPool {
    async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<Document>, ChonkitError> {
        Ok(sqlx::query_as!(
            Document,
            "SELECT id, name, path, ext, hash, src, label, tags, created_at, updated_at
             FROM documents 
             WHERE id = $1",
            id
        )
        .fetch_optional(self)
        .await?)
    }

    async fn get_config_by_id(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<DocumentConfig>, ChonkitError> {
        Ok(sqlx::query_as!(
            SelectDocumentConfig,
            r#"
            SELECT 
                d.id,
                d.name,
                d.path,
                d.ext,
                d.hash,
                d.src,
                c.config AS "chunk_config: Option<Json<Chunker>>",
                p.config AS "parse_config: _"
            FROM documents d 
            LEFT JOIN chunkers c ON c.document_id = d.id
            LEFT JOIN parsers p ON p.document_id = d.id
            WHERE d.id = $1"#,
            id
        )
        .fetch_optional(self)
        .await?
        .map(DocumentConfig::from))
    }

    async fn get_by_path(&self, path: &str) -> Result<Option<Document>, ChonkitError> {
        sqlx::query_as!(
            Document,
            "SELECT id, name, path, ext, hash, src, label, tags, created_at, updated_at 
             FROM documents 
             WHERE path = $1",
            path
        )
        .fetch_optional(self)
        .await
        .map_err(ChonkitError::from)
    }

    async fn get_path(&self, id: uuid::Uuid) -> Result<Option<String>, ChonkitError> {
        Ok(sqlx::query!("SELECT path FROM documents WHERE id = $1", id)
            .fetch_optional(self)
            .await?
            .map(|el| el.path))
    }

    async fn get_by_hash(&self, hash: &str) -> Result<Option<Document>, ChonkitError> {
        sqlx::query_as!(
            Document,
            "SELECT id, name, path, ext, hash, src, label, tags, created_at, updated_at 
             FROM documents 
             WHERE hash = $1",
            hash
        )
        .fetch_optional(self)
        .await
        .map_err(ChonkitError::from)
    }

    async fn list(
        &self,
        p: Pagination,
        _src: Option<&str>,
    ) -> Result<List<Document>, ChonkitError> {
        // TODO: implement `src` filter
        let total = sqlx::query!("SELECT COUNT(id) FROM documents")
            .fetch_one(self)
            .await
            .map(|row| row.count.map(|count| count as usize))?;

        let (limit, offset) = p.to_limit_offset();

        let documents = sqlx::query_as!(
            Document,
            r#"SELECT id, name, path, ext, hash, src, label, tags, created_at, updated_at
                   FROM documents
                   LIMIT $1
                   OFFSET $2
                "#,
            limit,
            offset,
        )
        .fetch_all(self)
        .await?;

        Ok(List::new(total, documents))
    }

    async fn insert(&self, params: DocumentInsert<'_>) -> Result<Document, ChonkitError> {
        let DocumentInsert {
            id,
            name,
            path,
            ext,
            src,
            hash,
            label,
            tags,
        } = params;

        sqlx::query_as!(
            Document,
            "INSERT INTO documents(id, name, path, ext, hash, src, label, tags)
             VALUES($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, name, path, ext, hash, src, label, tags, created_at, updated_at",
            id,
            name,
            path,
            ext.to_string(),
            hash,
            src,
            label,
            tags.as_deref(),
        )
        .fetch_one(self)
        .await
        .map_err(ChonkitError::from)
    }

    async fn update(
        &self,
        id: uuid::Uuid,
        update: DocumentUpdate<'_>,
    ) -> Result<u64, ChonkitError> {
        let DocumentUpdate { name, label, tags } = update;

        let result = sqlx::query!(
            r#"
            UPDATE documents SET 
            name = $1,
            label = $2,
            tags = $3
            WHERE id = $4 
        "#,
            name.as_ref(),
            label.as_ref(),
            tags.as_deref(),
            id
        )
        .execute(self)
        .await?;

        Ok(result.rows_affected())
    }

    async fn remove_by_id(&self, id: uuid::Uuid) -> Result<u64, ChonkitError> {
        let result = sqlx::query!("DELETE FROM documents WHERE id = $1", id)
            .execute(self)
            .await?;
        Ok(result.rows_affected())
    }

    async fn remove_by_path(&self, path: &str) -> Result<u64, ChonkitError> {
        let result = sqlx::query!("DELETE FROM documents WHERE path = $1", path)
            .execute(self)
            .await?;
        Ok(result.rows_affected())
    }

    async fn get_chunk_config(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<DocumentChunkConfig>, ChonkitError> {
        Ok(sqlx::query_as!(
            SelectConfig::<Chunker>,
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
        .fetch_optional(self)
        .await?
        .map(DocumentChunkConfig::from))
    }

    async fn get_parse_config(
        &self,
        id: uuid::Uuid,
    ) -> Result<Option<DocumentParseConfig>, ChonkitError> {
        Ok(sqlx::query_as!(
            SelectConfig::<ParseConfig>,
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
        .fetch_optional(self)
        .await?
        .map(DocumentParseConfig::from))
    }

    async fn upsert_chunk_config(
        &self,
        document_id: uuid::Uuid,
        chunker: Chunker,
    ) -> Result<DocumentChunkConfig, ChonkitError> {
        let config = InsertConfig::new(document_id, chunker);

        let InsertConfig {
            id,
            document_id,
            config,
        } = config;

        let config = sqlx::query_as!(
            SelectConfig::<Chunker>,
            r#"INSERT INTO chunkers
                (id, document_id, config)
             VALUES
                ($1, $2, $3)
             ON CONFLICT(document_id) DO UPDATE SET config = $3
             RETURNING
                id, document_id, config AS "config: _", created_at, updated_at
            "#,
            id,
            document_id,
            config as Json<Chunker>,
        )
        .fetch_one(self)
        .await?;

        Ok(DocumentChunkConfig::from(config))
    }

    async fn upsert_parse_config(
        &self,
        document_id: uuid::Uuid,
        config: ParseConfig,
    ) -> Result<DocumentParseConfig, ChonkitError> {
        let config = InsertConfig::new(document_id, config);

        let InsertConfig {
            id,
            document_id,
            config,
        } = config;

        let config = sqlx::query_as!(
            SelectConfig::<ParseConfig>,
            r#"INSERT INTO parsers
                (id, document_id, config)
             VALUES
                ($1, $2, $3)
             ON CONFLICT(document_id) DO UPDATE SET config = $3
             RETURNING
                id, document_id, config AS "config: _", created_at, updated_at"#,
            id,
            document_id,
            config as Json<ParseConfig>,
        )
        .fetch_one(self)
        .await?;

        Ok(DocumentParseConfig::from(config))
    }
}

// Private dtos.

struct InsertConfig<T: Serialize> {
    id: uuid::Uuid,
    document_id: uuid::Uuid,
    config: sqlx::types::Json<T>,
}

impl<T> InsertConfig<T>
where
    T: Serialize,
{
    fn new(document_id: uuid::Uuid, config: T) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            document_id,
            config: Json(config),
        }
    }
}

struct SelectDocumentConfig {
    id: uuid::Uuid,
    name: String,
    path: String,
    ext: String,
    hash: String,
    src: String,
    chunk_config: Option<Json<Chunker>>,
    parse_config: Option<Json<ParseConfig>>,
}

impl From<SelectDocumentConfig> for DocumentConfig {
    fn from(
        SelectDocumentConfig {
            id,
            name,
            path,
            ext,
            hash,
            src,
            chunk_config,
            parse_config,
        }: SelectDocumentConfig,
    ) -> Self {
        Self {
            id,
            name,
            path,
            ext,
            hash,
            src,
            chunk_config: chunk_config.map(|c| c.0),
            parse_config: parse_config.map(|c| c.0),
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

impl From<SelectConfig<Chunker>> for DocumentChunkConfig {
    fn from(value: SelectConfig<Chunker>) -> Self {
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

impl From<SelectConfig<ParseConfig>> for DocumentParseConfig {
    fn from(value: SelectConfig<ParseConfig>) -> Self {
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

    use crate::{
        app::test::{init_postgres, PostgresContainer},
        core::{
            chunk::Chunker,
            model::document::{DocumentInsert, DocumentType},
            repo::document::DocumentRepo,
        },
    };
    use sqlx::PgPool;
    use suitest::before_all;

    #[before_all]
    async fn setup() -> (PgPool, PostgresContainer) {
        let (postgres, pg_img) = init_postgres().await;
        (postgres, pg_img)
    }

    #[test]
    async fn inserting_document_works(repo: PgPool) {
        let doc = DocumentInsert::new(
            "My file",
            "path/to/file",
            DocumentType::Text,
            "SHA256",
            "fs",
        );
        let doc = repo.insert(doc).await.unwrap();
        let doc = repo.get_by_id(doc.id).await.unwrap().unwrap();

        assert_eq!("My file", doc.name);
        assert_eq!("path/to/file", doc.path);
        assert_eq!("txt", doc.ext);

        repo.remove_by_id(doc.id).await.unwrap();

        let doc = repo.get_by_id(doc.id).await.unwrap();

        assert!(doc.is_none());
    }

    #[test]
    async fn inserting_chunk_config_works(repo: PgPool) {
        let doc = DocumentInsert::new(
            "My file",
            "path/to/file/2",
            DocumentType::Text,
            "Other hash",
            "fs",
        );
        let doc = repo.insert(doc).await.unwrap();
        let chunker = Chunker::sliding(420, 69).unwrap();
        repo.upsert_chunk_config(doc.id, chunker.clone())
            .await
            .unwrap();
        let config = repo.get_chunk_config(doc.id).await.unwrap().unwrap();
        let Chunker::Sliding(sliding) = config.config else {
            panic!("invalid config variant");
        };
        let Chunker::Sliding(chunker) = chunker else {
            panic!("the impossible happened");
        };
        assert_eq!(chunker.config.size, sliding.config.size);
        assert_eq!(chunker.config.overlap, sliding.config.overlap);
        repo.remove_by_id(doc.id).await.unwrap();
    }
}
