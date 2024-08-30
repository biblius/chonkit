use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Serialize};

use crate::core::{chunk::ChunkConfig, document::parser::DocumentParser};

/// Main config model for the `doc_configs` table.
#[derive(Debug, Serialize, Default)]
pub struct DocumentConfig {
    /// Primary key.
    pub id: uuid::Uuid,

    /// References the document which this config belongs to.
    pub document_id: uuid::Uuid,

    /// JSON string of the chunking configuration.
    pub chunk_config: Option<serde_json::Value>,

    /// JSON string of the parsing configuration.
    pub parse_config: Option<serde_json::Value>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DocumentConfigInsert {
    pub id: uuid::Uuid,
    pub document_id: uuid::Uuid,
    pub chunk_config: Option<serde_json::Value>,
    pub parse_config: Option<serde_json::Value>,
}

impl DocumentConfigInsert {
    pub fn new(document_id: uuid::Uuid) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            document_id,
            chunk_config: None,
            parse_config: None,
        }
    }

    pub fn with_chunk_config(mut self, cfg: ChunkConfig) -> Result<Self, serde_json::Error> {
        self.chunk_config = Some(serde_json::to_value(cfg)?);
        Ok(self)
    }

    pub fn with_parse_config(
        mut self,
        parser: impl DocumentParser + Serialize + DeserializeOwned,
    ) -> Result<Self, serde_json::Error> {
        self.parse_config = Some(serde_json::to_value(parser)?);
        Ok(self)
    }
}
