use super::document::store::FsDocumentStore;
use crate::{
    core::{chunk::Chunker, document::store::DocumentStore, embedder::Embedder, vector::VectorDb},
    error::ChonkitError,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};

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

    pub fn get_configuration(&self) -> Result<AppConfig, ChonkitError> {
        let mut embedding_providers = HashMap::new();
        let mut default_chunkers = vec![Chunker::sliding_default(), Chunker::snapping_default()];

        for provider_str in EMBEDDING_PROVIDERS {
            let provider = (*provider_str).try_into()?;
            let embedder = self.embedder(provider);

            default_chunkers.push(Chunker::semantic_default(embedder.clone()));

            let models = embedder.list_embedding_models().into_iter().collect();
            embedding_providers.insert(provider_str.to_string(), models);
        }

        Ok(AppConfig {
            vector_providers: VECTOR_PROVIDERS.iter().map(|s| s.to_string()).collect(),
            embedding_providers,
            default_chunkers,
        })
    }
}

#[cfg_attr(feature = "http", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// A list of available vector providers.
    pub vector_providers: Vec<String>,

    /// A map of available embedding providers, their models and the respective model sizes.
    pub embedding_providers: HashMap<String, HashMap<String, usize>>,

    /// A list of default chunking configurations.
    pub default_chunkers: Vec<Chunker>,
}

/// Creates a provider enum and its TryFrom <String> and <&str> implementations.
///
/// Implements functions for ServiceState to easily get an instance of whatever
/// the provider is for.
///
/// Additionally, creates a constant with the given feature literals so we can easily list them
/// to the client.
macro_rules! provider {
    (
        $name:ident : $return_ty:path : $fn_name:ident,
        $($feature:expr => $provider:ident => $state_id:ident),*
        $(,)?;
        $constant_name:ident
    ) => {
            #[derive(Debug, Clone, Deserialize)]
            pub enum $name {
                $(
                    #[cfg(feature = $feature)]
                    $provider,
                )*
            }

            impl TryFrom<&str> for $name {
                type Error = ChonkitError;

                fn try_from(provider: &str) -> Result<Self, Self::Error> {
                    match provider {
                        $(
                            #[cfg(feature = $feature)]
                            $feature => Ok(Self::$provider),
                        )*
                        _ => Err(ChonkitError::InvalidProvider(format!(
                            "Invalid provider: {provider}"
                        ))),
                    }
                }
            }

            impl TryFrom<String> for $name {
                type Error = ChonkitError;

                fn try_from(provider: String) -> Result<Self, Self::Error> {
                    provider.as_str().try_into()
                }
            }

            impl ServiceState {
                pub fn $fn_name(&self, provider: $name) -> Arc<dyn $return_ty + Send + Sync> {
                    match provider {
                        $(
                            #[cfg(feature = $feature)]
                            $name::$provider => self.$state_id.clone(),
                        )*
                    }
                }
            }

            pub(in $crate::app) const $constant_name: &[&str] = &[
                $(
                    #[cfg(feature = $feature)]
                    $feature
                ),*
            ];
    };
}

provider! {
    EmbeddingProvider:Embedder:embedder,
        "fembed" => FastEmbed => fastembed,
        "openai" => OpenAi => openai;
    EMBEDDING_PROVIDERS
}

provider! {
    VectorProvider:VectorDb:vector_db,
        "qdrant" => Qdrant => qdrant,
        "weaviate" => Weaviate => weaviate;
    VECTOR_PROVIDERS
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
