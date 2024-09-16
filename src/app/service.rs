use super::document::store::FsDocumentStore;
use crate::{
    core::{document::store::DocumentStore, embedder::Embedder, vector::VectorDb},
    error::ChonkitError,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;

pub mod document;
pub mod vector;

#[derive(Clone)]
pub struct ServiceState {
    pub postgres: PgPool,

    pub fs_store: Arc<FsDocumentStore>,

    #[cfg(feature = "openai")]
    pub openai: Arc<super::embedder::openai::OpenAiEmbeddings>,

    #[cfg(feature = "fembed")]
    pub fastembed: Arc<super::embedder::fastembed::FastEmbedder>,

    #[cfg(feature = "qdrant")]
    pub qdrant: Arc<super::vector::qdrant::QdrantDb>,

    #[cfg(feature = "weaviate")]
    pub weaviate: Arc<super::vector::weaviate::WeaviateDb>,
}

impl ServiceState {
    pub fn store(&self, provider: DocumentStoreProvider) -> Arc<dyn DocumentStore + Send + Sync> {
        match provider {
            DocumentStoreProvider::Fs => self.fs_store.clone(),
        }
    }

    pub fn vector_db(&self, provider: VectorProvider) -> Arc<dyn VectorDb + Send + Sync> {
        match provider {
            #[cfg(feature = "qdrant")]
            VectorProvider::Qdrant => self.qdrant.clone(),

            #[cfg(feature = "weaviate")]
            VectorProvider::Weaviate => self.weaviate.clone(),
        }
    }

    pub fn embedder(&self, provider: EmbeddingProvider) -> Arc<dyn Embedder + Send + Sync> {
        match provider {
            #[cfg(feature = "fembed")]
            EmbeddingProvider::FastEmbed => self.fastembed.clone(),

            #[cfg(feature = "openai")]
            EmbeddingProvider::OpenAi => self.openai.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum VectorProvider {
    #[cfg(feature = "qdrant")]
    Qdrant,

    #[cfg(feature = "weaviate")]
    Weaviate,
}

impl TryFrom<String> for VectorProvider {
    type Error = ChonkitError;

    fn try_from(provider: String) -> Result<Self, Self::Error> {
        provider.as_str().try_into()
    }
}

impl TryFrom<&str> for VectorProvider {
    type Error = ChonkitError;

    fn try_from(provider: &str) -> Result<Self, Self::Error> {
        match provider {
            #[cfg(feature = "qdrant")]
            "qdrant" => Ok(Self::Qdrant),

            #[cfg(feature = "weaviate")]
            "weaviate" => Ok(Self::Weaviate),

            _ => Err(ChonkitError::InvalidProvider(format!(
                "Invalid vector provider: {provider}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum DocumentStoreProvider {
    Fs,
}

impl TryFrom<String> for DocumentStoreProvider {
    type Error = ChonkitError;

    fn try_from(provider: String) -> Result<Self, Self::Error> {
        provider.as_str().try_into()
    }
}

impl TryFrom<&str> for DocumentStoreProvider {
    type Error = ChonkitError;

    fn try_from(provider: &str) -> Result<Self, Self::Error> {
        match provider {
            "fs" => Ok(Self::Fs),

            _ => Err(ChonkitError::InvalidProvider(format!(
                "Invalid document store provider: {provider}"
            ))),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum EmbeddingProvider {
    #[cfg(feature = "fembed")]
    FastEmbed,
    #[cfg(feature = "openai")]
    OpenAi,
}

impl TryFrom<String> for EmbeddingProvider {
    type Error = ChonkitError;

    fn try_from(provider: String) -> Result<Self, Self::Error> {
        provider.as_str().try_into()
    }
}

impl TryFrom<&str> for EmbeddingProvider {
    type Error = ChonkitError;

    fn try_from(provider: &str) -> Result<Self, Self::Error> {
        match provider {
            #[cfg(feature = "fembed")]
            "fastembed" => Ok(Self::FastEmbed),

            #[cfg(feature = "openai")]
            "openai" => Ok(Self::OpenAi),

            _ => Err(ChonkitError::InvalidProvider(format!(
                "Invalid embedding provider: {provider}"
            ))),
        }
    }
}
