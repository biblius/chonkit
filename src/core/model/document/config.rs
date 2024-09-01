use crate::core::{chunk::ChunkConfig, document::parser::Parser};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Main config model for the `chunkers` table.
#[derive(Debug, Serialize)]
pub struct DocumentChunkConfig {
    /// Primary key.
    pub id: uuid::Uuid,
    /// References the document which this config belongs to.
    pub document_id: uuid::Uuid,
    /// JSON string of the chunking configuration.
    pub config: ChunkConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DocumentChunkConfigInsert {
    pub id: uuid::Uuid,
    pub document_id: uuid::Uuid,
    pub config: ChunkConfig,
}

impl DocumentChunkConfigInsert {
    pub fn new(document_id: uuid::Uuid, config: ChunkConfig) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            document_id,
            config,
        })
    }
}

/// Main config model for the `parsers` table.
#[derive(Debug, Serialize)]
pub struct DocumentParseConfig {
    /// Primary key.
    pub id: uuid::Uuid,
    /// References the document which this config belongs to.
    pub document_id: uuid::Uuid,
    /// JSON string of the parsing configuration.
    pub config: Parser,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DocumentParseConfigInsert {
    pub id: uuid::Uuid,
    pub document_id: uuid::Uuid,
    pub config: Parser,
}

impl DocumentParseConfigInsert {
    pub fn new(document_id: uuid::Uuid, parser: Parser) -> Result<Self, serde_json::Error> {
        Ok(Self {
            id: uuid::Uuid::new_v4(),
            document_id,
            config: parser,
        })
    }
}
