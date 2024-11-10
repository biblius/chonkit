use super::{document::store::DocumentStore, embedder::Embedder, vector::VectorDb};
use crate::error::ChonkitError;
use std::sync::Arc;

type DynVectorDb = Arc<dyn VectorDb + Send + Sync>;
type DynEmbedder = Arc<dyn Embedder + Send + Sync>;
type DynDocumentStore = Arc<dyn DocumentStore + Send + Sync>;

/// Provider factories are used to decouple concrete implementations from the business logic.
///
/// The concrete instances are always obtained from aggregate roots, i.e. [Documents][crate::core::model::document::Document]
/// or [Collections][crate::core::model::collection::Collection].
pub trait ProviderFactory<T> {
    fn get_provider(&self, input: &str) -> Result<T, ChonkitError>;
}

/// Holds the factories for all available providers.
#[derive(Clone)]
pub struct ProviderState {
    /// Vector database provider.
    pub vector: Arc<dyn ProviderFactory<DynVectorDb> + Send + Sync>,

    /// Embedding provider.
    pub embedding: Arc<dyn ProviderFactory<DynEmbedder> + Send + Sync>,

    /// Document storage provider.
    pub store: Arc<dyn ProviderFactory<DynDocumentStore> + Send + Sync>,
}
