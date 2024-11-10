use super::{
    batch::{BatchEmbedder, BatchEmbedderHandle},
    document::store::FsDocumentStore,
};
use crate::{
    core::{
        chunk::Chunker,
        document::store::{DocumentStore, DocumentSync},
        embedder::Embedder,
        provider::{ProviderFactory, ProviderState},
        repo::document::DocumentRepo,
        service::{document::DocumentService, vector::VectorService},
        vector::VectorDb,
    },
    error::ChonkitError,
};
use serde::Serialize;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tracing_subscriber::EnvFilter;

pub mod document;
pub mod vector;

#[derive(Clone)]
pub struct GlobalState {
    pub app_state: AppState,
    pub service_state: ServiceState,
    pub batch_embedder: BatchEmbedderHandle,
}

#[derive(Clone)]
pub struct AppState {
    pub postgres: PgPool,

    pub vector_provider: Arc<VectorStoreProvider>,

    pub embedding_provider: Arc<EmbeddingProvider>,

    pub document_provider: Arc<DocumentStoreProvider>,
}

impl AppState {
    /// Load the application state using the provided configuration.
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

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from(args.log()))
            .init();

        let postgres = crate::app::repo::pg::init(&args.db_url()).await;

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
        let qdrant = crate::app::vector::qdrant::init(&args.qdrant_url());

        #[cfg(feature = "weaviate")]
        let weaviate = crate::app::vector::weaviate::init(&args.weaviate_url());

        let vector_provider = Arc::new(VectorStoreProvider {
            #[cfg(feature = "qdrant")]
            qdrant,

            #[cfg(feature = "weaviate")]
            weaviate,
        });

        let embedding_provider = Arc::new(EmbeddingProvider {
            #[cfg(feature = "fembed")]
            fastembed,

            #[cfg(feature = "openai")]
            openai,
        });

        let document_provider = Arc::new(DocumentStoreProvider { fs_store });

        Self {
            postgres,
            document_provider,
            vector_provider,
            embedding_provider,
        }
    }

    /// Get an instance of a document sync implementation for the given repository.
    pub fn syncer<T>(
        &self,
        input: &str,
    ) -> Result<Arc<dyn DocumentSync<T> + Send + Sync>, ChonkitError>
    where
        T: DocumentRepo + Send + Sync,
    {
        match input {
            _ if self.document_provider.fs_store.id() == input => {
                Ok(self.document_provider.fs_store.clone())
            }
            _ => Err(ChonkitError::InvalidProvider(input.to_string())),
        }
    }

    /// Used for metadata display.
    pub async fn get_configuration(&self) -> Result<AppConfig, ChonkitError> {
        let mut embedding_providers = HashMap::new();
        let mut default_chunkers = vec![Chunker::sliding_default(), Chunker::snapping_default()];

        for provider in EMBEDDING_PROVIDERS {
            let embedder = self.embedding_provider.get_provider(provider)?;

            default_chunkers.push(Chunker::semantic_default(embedder.clone()));

            let models = embedder
                .list_embedding_models()
                .await?
                .into_iter()
                .collect();

            embedding_providers.insert(provider.to_string(), models);
        }

        let mut document_providers = vec![
            /* Temporary, until there is more providers. */ "fs".to_string(),
        ];
        document_providers.extend(
            DOCUMENT_PROVIDERS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        );

        Ok(AppConfig {
            vector_providers: VECTOR_PROVIDERS.iter().map(|s| s.to_string()).collect(),
            embedding_providers,
            default_chunkers,
            document_providers,
        })
    }

    pub fn to_provider_state(&self) -> ProviderState {
        ProviderState {
            vector: self.vector_provider.clone(),
            embedding: self.embedding_provider.clone(),
            store: self.document_provider.clone(),
        }
    }
}

pub fn spawn_batch_embedder(state: ServiceState) -> BatchEmbedderHandle {
    let (tx, rx) = tokio::sync::mpsc::channel(128);
    BatchEmbedder::new(rx, state).start();
    tx
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// A list of available vector providers.
    pub vector_providers: Vec<String>,

    /// A map of available embedding providers, their models and their respective model sizes.
    pub embedding_providers: HashMap<String, HashMap<String, usize>>,

    /// A list of available document storage providers.
    pub document_providers: Vec<String>,

    /// A list of default chunking configurations.
    pub default_chunkers: Vec<Chunker>,
}

/// Implements functions for `$target` to easily get an instance of whatever
/// the provider is for, i.e. `$provider_out`.
///
/// Additionally, creates a constant with the given feature literals so we can easily list them
/// to the client.
macro_rules! provider {
    (
        $target:ident -> $provider_out:ident,
        $($($feature:literal =>)? $provider:ident => $state_id:ident),*
        $(,)?;
        $constant_name:ident
    ) => {
            impl ProviderFactory<Arc<dyn $provider_out + Send + Sync>> for $target {
                /// AUTO-GENERATED BY THE `provider!` MACRO.
                /// SEE [crate::app::state] FOR MORE DETAILS.
                /// Obtain the provider for the given enum variant from the application state.
                fn get_provider(&self, input: &str) -> Result<Arc<dyn $provider_out + Send + Sync>, ChonkitError> {
                    match input {
                        $(
                            $(#[cfg(feature = $feature)])?
                            _ if self.$state_id.id() == input => Ok(self.$state_id.clone()),
                        )*
                        _ => Err(ChonkitError::InvalidProvider(input.to_string()))
                    }
                }
            }

            /// AUTO-GENERATED BY THE `provider!` MACRO.
            /// SEE [crate::app::state] FOR MORE DETAILS.
            /// A list of available providers for a given functionality.
            pub(in $crate::app) const $constant_name: &[&str] = &[
                $(
                    $(#[cfg(feature = $feature)])?
                    $($feature)?
                ),*
            ];
    };
}

provider! {
    EmbeddingProvider -> Embedder,
        "fembed" => FastEmbed => fastembed,
        "openai" => OpenAi => openai;
    EMBEDDING_PROVIDERS
}

provider! {
    VectorStoreProvider -> VectorDb,
        "qdrant" => Qdrant => qdrant,
        "weaviate" => Weaviate => weaviate;
    VECTOR_PROVIDERS
}

provider! {
    DocumentStoreProvider -> DocumentStore,
        FsDocumentStore => fs_store;
    DOCUMENT_PROVIDERS
}

#[derive(Clone)]
pub struct ServiceState {
    pub document: DocumentService<PgPool>,
    pub vector: VectorService<PgPool>,
}

impl ServiceState {
    pub fn from_app_state(state: &AppState) -> Self {
        Self::new(state.postgres.clone(), state.to_provider_state())
    }

    fn new(repository: PgPool, providers: ProviderState) -> Self {
        let document = DocumentService::new(repository.clone(), providers.clone());
        let vector = VectorService::new(repository, providers);
        Self { document, vector }
    }
}

/// Provides concrete implementations of [Embedder] for each provider.
#[derive(Clone)]
pub struct EmbeddingProvider {
    #[cfg(feature = "openai")]
    pub openai: Arc<super::embedder::openai::OpenAiEmbeddings>,

    #[cfg(feature = "fembed")]
    pub fastembed: Arc<super::embedder::fastembed::FastEmbedder>,
}

/// Provides concrete implementations of [VectorDb] for each provider.
#[derive(Clone)]
pub struct VectorStoreProvider {
    #[cfg(feature = "qdrant")]
    pub qdrant: super::vector::qdrant::QdrantDb,

    #[cfg(feature = "weaviate")]
    pub weaviate: super::vector::weaviate::WeaviateDb,
}

/// Provides concrete implementations of [DocumentStore] for each provider.
#[derive(Clone)]
pub struct DocumentStoreProvider {
    pub fs_store: Arc<FsDocumentStore>,
}

// #[macro_export]
// macro_rules! conditional_state {
//     (
//         $( $id:ident = $provider:ident { $( $feature:literal : $state_id:ident ),* $(,)? } $(,)? )*
// ) => {
//      $(
//
//      )*
// };
// }
