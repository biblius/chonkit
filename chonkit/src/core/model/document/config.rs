use crate::core::{chunk::ChunkConfig, document::parser::ParseConfig};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Main config model for the `chunkers` table.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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

/// Main config model for the `parsers` table.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentParseConfig {
    /// Primary key.
    pub id: uuid::Uuid,
    /// References the document which this config belongs to.
    pub document_id: uuid::Uuid,
    /// JSON string of the parsing configuration.
    pub config: ParseConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
