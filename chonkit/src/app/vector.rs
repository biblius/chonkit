use crate::{
    config::{
        DEFAULT_COLLECTION_EMBEDDING_MODEL, DEFAULT_COLLECTION_EMBEDDING_PROVIDER,
        DEFAULT_COLLECTION_ID, DEFAULT_COLLECTION_NAME, DEFAULT_COLLECTION_SIZE,
    },
    core::vector::CreateVectorCollection,
};

impl Default for CreateVectorCollection<'static> {
    fn default() -> Self {
        Self {
            collection_id: DEFAULT_COLLECTION_ID,
            name: DEFAULT_COLLECTION_NAME,
            size: DEFAULT_COLLECTION_SIZE,
            embedding_provider: DEFAULT_COLLECTION_EMBEDDING_PROVIDER,
            embedding_model: DEFAULT_COLLECTION_EMBEDDING_MODEL,
        }
    }
}

#[cfg(feature = "qdrant")]
pub mod qdrant;

#[cfg(feature = "weaviate")]
pub mod weaviate;
