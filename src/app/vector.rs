const CONTENT_PROPERTY: &str = "content";
const DOCUMENT_ID_PROPERTY: &str = "document_id";

#[cfg(feature = "qdrant")]
pub mod qdrant;

#[cfg(feature = "weaviate")]
pub mod weaviate;
