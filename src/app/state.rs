use super::{
    batch::{BatchEmbedder, BatchEmbedderHandle},
    document::store::FsDocumentStore,
};
use crate::{
    core::{
        chunk::Chunker,
        document::store::{DocumentStore, DocumentSync},
        embedder::Embedder,
        repo::document::DocumentRepo,
        vector::VectorDb,
    },
    error::ChonkitError,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tracing_subscriber::EnvFilter;

pub mod document;
pub mod vector;

#[derive(Clone)]
pub struct AppState {
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

impl AppState {
    pub async fn new(args: &crate::config::StartArgs) -> Self {
        // Ensures the dynamic library is loaded and panics if it isn't
        pdfium_render::prelude::Pdfium::default();

        #[cfg(all(
            feature = "fembed",
            not(any(feature = "fe-local", feature = "fe-remote"))
        ))]
        compile_error!(
            "either `fe-local` or `fe-remote` must be enabled when running with `fembed`"
        );

        let db_url = args.db_url();

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from(args.log()))
            .init();

        let postgres = crate::app::repo::pg::init(&db_url).await;

        let fs_store = Arc::new(crate::app::document::store::FsDocumentStore::new(
            &args.upload_path(),
        ));

        #[cfg(feature = "fe-local")]
        tracing::info!(
            "Cuda available: {:?}",
            ort::ExecutionProvider::is_available(&ort::CUDAExecutionProvider::default())
        );

        #[cfg(feature = "fe-local")]
        let fastembed = Arc::new(crate::app::embedder::fastembed::FastEmbedder::new());

        #[cfg(feature = "fe-remote")]
        let fastembed = Arc::new(crate::app::embedder::fastembed::FastEmbedder::new(
            args.fembed_url(),
        ));

        #[cfg(feature = "openai")]
        let openai = Arc::new(crate::app::embedder::openai::OpenAiEmbeddings::new(
            &args.open_ai_key(),
        ));

        #[cfg(feature = "qdrant")]
        let qdrant = Arc::new(crate::app::vector::qdrant::init(&args.qdrant_url()));

        #[cfg(feature = "weaviate")]
        let weaviate = Arc::new(crate::app::vector::weaviate::init(&args.weaviate_url()));

        Self {
            postgres,

            fs_store,

            #[cfg(feature = "fembed")]
            fastembed,

            #[cfg(feature = "openai")]
            openai,

            #[cfg(feature = "qdrant")]
            qdrant,

            #[cfg(feature = "weaviate")]
            weaviate,
        }
    }

    pub fn store(&self, provider: DocumentStoreProvider) -> Arc<dyn DocumentStore + Send + Sync> {
        match provider {
            DocumentStoreProvider::Fs => self.fs_store.clone(),
        }
    }

    pub fn syncer<T>(
        &self,
        provider: DocumentStoreProvider,
    ) -> Arc<dyn DocumentSync<T> + Send + Sync>
    where
        T: DocumentRepo + Send + Sync,
    {
        match provider {
            DocumentStoreProvider::Fs => self.fs_store.clone(),
        }
    }

    pub async fn get_configuration(&self) -> Result<AppConfig, ChonkitError> {
        let mut embedding_providers = HashMap::new();
        let mut default_chunkers = vec![Chunker::sliding_default(), Chunker::snapping_default()];

        for provider_str in EMBEDDING_PROVIDERS {
            let provider = (*provider_str).try_into()?;
            let embedder = self.embedder(provider);

            default_chunkers.push(Chunker::semantic_default(embedder.clone()));

            let models = embedder
                .list_embedding_models()
                .await?
                .into_iter()
                .collect();
            embedding_providers.insert(provider_str.to_string(), models);
        }

        Ok(AppConfig {
            vector_providers: VECTOR_PROVIDERS.iter().map(|s| s.to_string()).collect(),
            embedding_providers,
            default_chunkers,
        })
    }
}

pub fn spawn_batch_embedder(state: AppState) -> BatchEmbedderHandle {
    let (tx, rx) = tokio::sync::mpsc::channel(128);
    BatchEmbedder::new(rx, state).start();
    tx
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
/// Implements functions for AppState to easily get an instance of whatever
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

            impl AppState {
                /// AUTO-GENERATED BY THE `provider!` MACRO.
                /// SEE [crate::app::state] FOR MORE DETAILS.
                /// Obtain the provider for the given enum variant from the application state.
                pub fn $fn_name(&self, provider: $name) -> Arc<dyn $return_ty + Send + Sync> {
                    match provider {
                        $(
                            #[cfg(feature = $feature)]
                            $name::$provider => self.$state_id.clone(),
                        )*
                    }
                }
            }

            /// AUTO-GENERATED BY THE `provider!` MACRO.
            /// SEE [crate::app::state] FOR MORE DETAILS.
            /// Represents the available providers for a given functionality.
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
