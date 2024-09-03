use crate::core::chunk::ChunkConfig;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct CreateCollectionPayload {
    pub name: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchPayload {
    pub model: String,
    pub query: String,
    pub collection: String,
    pub limit: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct EmbedPayload {
    /// Document ID.
    pub id: uuid::Uuid,
    /// Vector collection
    pub collection: String,
    /// Chunking config.
    pub config: ChunkConfig,
}
