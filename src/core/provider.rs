use super::{document::store::DocumentStore, embedder::Embedder, vector::VectorDb};
use crate::error::ChonkitError;
use std::sync::Arc;

pub type DynVectorDb = Arc<dyn VectorDb + Send + Sync>;
pub type DynEmbedder = Arc<dyn Embedder + Send + Sync>;
pub type DynDocumentStore = Arc<dyn DocumentStore + Send + Sync>;

pub trait ProviderFactory<T> {
    fn provider(&self, input: &str) -> Result<T, ChonkitError>;
}

#[derive(Clone)]
pub struct ProviderState {
    pub vector: Arc<dyn ProviderFactory<DynVectorDb> + Send + Sync>,
    pub embedding: Arc<dyn ProviderFactory<DynEmbedder> + Send + Sync>,
    pub store: Arc<dyn ProviderFactory<DynDocumentStore> + Send + Sync>,
}
