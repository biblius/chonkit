use crate::core::{chunk::Chunker, document::parser::ParseConfig};
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
    pub config: Chunker,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Main config model for the `parsers` table.
#[derive(Debug, Serialize)]
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
