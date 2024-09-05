use crate::core::{chunk::Chunker, document::parser::ParseConfig};
use serde::Deserialize;

/// DTO used for previewing chunks.
#[derive(Debug, Deserialize, Default)]
pub struct ChunkPreviewPayload {
    pub parser: Option<ParseConfig>,
    pub chunker: Option<Chunker>,
}
