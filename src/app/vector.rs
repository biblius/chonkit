use serde::Deserialize;

#[cfg(feature = "qdrant")]
pub mod qdrant;

#[cfg(feature = "weaviate")]
pub mod weaviate;

#[derive(Clone, Debug, Deserialize)]
pub enum VectorProvider {
    Qdrant,
    Weaviate,
}
