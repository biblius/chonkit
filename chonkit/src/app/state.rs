use super::{
    batch::{BatchEmbedder, BatchEmbedderHandle},
    document::store::FsDocumentStore,
};
use crate::{
    core::{
        chunk::ChunkConfig,
        document::store::DocumentStore,
        embedder::Embedder,
        provider::{ProviderFactory, ProviderState},
        service::{document::DocumentService, vector::VectorService},
        vector::VectorDb,
    },
    err,
    error::ChonkitError,
};
use serde::Serialize;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct AppState {
    /// Chonkit services.
    pub services: ServiceState,

    /// Handle for batch embedding documents.
    pub batch_embedder: BatchEmbedderHandle,

    /// Downstream service providers for chonkit services.
    /// Used for displaying some metadata and in tests.
    pub providers: AppProviderState,

    #[cfg(feature = "auth-vault")]
    pub vault: crate::app::auth::VaultAuthenticator,
}

impl AppState {
    /// Load the application state using the provided configuration.
    pub async fn new(args: &crate::config::StartArgs) -> Self {
        // Ensures the dynamic library is loaded and panics if it isn't
        pdfium_render::prelude::Pdfium::default();

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from(args.log()))
            .init();

        let postgres = crate::app::repo::pg::init(&args.db_url()).await;

        let vector_provider = Self::init_vector_providers(args);
        let embedding_provider = Self::init_embedding_providers(args);
        let document_provider = Self::init_document_providers(args);

        let providers = AppProviderState {
            database: postgres.clone(),
            vector: vector_provider,
            embedding: embedding_provider,
            document: document_provider,
        };

        let document = DocumentService::new(postgres.clone(), providers.clone().into());
        let vector = VectorService::new(postgres, providers.clone().into());

        document.create_default_document().await;
        for provider in providers.vector.list_provider_ids() {
            for e_provider in providers.embedding.list_provider_ids() {
                vector.create_default_collection(provider, e_provider).await;
            }
        }

        let service_state = ServiceState { document, vector };

        let batch_embedder = Self::spawn_batch_embedder(service_state.clone());

        #[cfg(feature = "auth-vault")]
        let vault = Self::init_vault(args).await;

        Self {
            services: service_state,
            batch_embedder,
            providers,
            #[cfg(feature = "auth-vault")]
            vault,
        }
    }

    #[cfg(feature = "auth-vault")]
    async fn init_vault(args: &crate::config::StartArgs) -> crate::app::auth::VaultAuthenticator {
        crate::app::auth::VaultAuthenticator::new(
            args.vault_url(),
            args.vault_role_id(),
            args.vault_secret_id(),
            args.vault_key_name(),
        )
        .await
    }

    fn init_vector_providers(args: &crate::config::StartArgs) -> Arc<VectorDbProvider> {
        let mut provider = VectorDbProvider::default();

        #[cfg(feature = "qdrant")]
        {
            let qdrant = crate::app::vector::qdrant::init(&args.qdrant_url());
            provider.register(qdrant.id(), qdrant);
        }

        #[cfg(feature = "weaviate")]
        {
            let weaviate = crate::app::vector::weaviate::init(&args.weaviate_url());
            provider.register(weaviate.id(), weaviate);
        }

        Arc::new(provider)
    }

    fn init_embedding_providers(_args: &crate::config::StartArgs) -> Arc<EmbeddingProvider> {
        #[cfg(not(any(feature = "fe-local", feature = "fe-remote", feature = "openai")))]
        compile_error!("one of `fe-local`, `fe-remote` or `openai` features must be enabled");

        let mut provider = EmbeddingProvider::default();

        #[cfg(feature = "fe-local")]
        {
            let fastembed =
                Arc::new(crate::app::embedder::fastembed::local::LocalFastEmbedder::new());
            provider.register(fastembed.id(), fastembed);
        }

        // Remote implementations take precedence. This will override the local implementation
        // in the provider state.
        #[cfg(feature = "fe-remote")]
        {
            let fastembed = Arc::new(
                crate::app::embedder::fastembed::remote::RemoteFastEmbedder::new(
                    _args.fembed_url(),
                ),
            );
            provider.register(fastembed.id(), fastembed);
        }

        #[cfg(feature = "openai")]
        {
            let openai = Arc::new(crate::app::embedder::openai::OpenAiEmbeddings::new(
                &_args.open_ai_key(),
            ));
            provider.register(openai.id(), openai);
        }

        Arc::new(provider)
    }

    fn init_document_providers(args: &crate::config::StartArgs) -> Arc<DocumentStoreProvider> {
        let mut provider = DocumentStoreProvider::default();

        let fs_store = Arc::new(FsDocumentStore::new(&args.upload_path()));
        provider.register(fs_store.id(), fs_store);

        Arc::new(provider)
    }

    fn spawn_batch_embedder(state: ServiceState) -> BatchEmbedderHandle {
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        BatchEmbedder::new(rx, state).start();
        tx
    }

    /// Used for metadata display.
    pub async fn get_configuration(&self) -> Result<AppConfig, ChonkitError> {
        let mut embedding_providers = HashMap::new();
        let mut default_chunkers = vec![
            ChunkConfig::sliding_default(),
            ChunkConfig::snapping_default(),
        ];

        for provider in self.providers.embedding.list_provider_ids() {
            let embedder = self.providers.embedding.get_provider(provider)?;
            let default_model = embedder.default_model().0;

            default_chunkers.push(ChunkConfig::semantic_default(
                embedder.id().to_string(),
                default_model,
            ));

            let models = embedder
                .list_embedding_models()
                .await?
                .into_iter()
                .collect();

            embedding_providers.insert(provider.to_string(), models);
        }

        let document_providers = self
            .providers
            .document
            .list_provider_ids()
            .iter()
            .map(|s| s.to_string())
            .collect();

        Ok(AppConfig {
            vector_providers: self
                .providers
                .vector
                .list_provider_ids()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            embedding_providers,
            default_chunkers,
            document_providers,
        })
    }

    #[cfg(test)]
    pub fn new_test(
        services: ServiceState,
        providers: AppProviderState,
        #[cfg(feature = "auth-vault")] vault: super::auth::VaultAuthenticator,
    ) -> Self {
        Self {
            services: services.clone(),
            providers,
            batch_embedder: Self::spawn_batch_embedder(services),
            #[cfg(feature = "auth-vault")]
            vault,
        }
    }
}

/// Concrete version of [ProviderState].
#[derive(Clone)]
pub struct AppProviderState {
    pub database: PgPool,
    pub vector: Arc<VectorDbProvider>,
    pub embedding: Arc<EmbeddingProvider>,
    pub document: Arc<DocumentStoreProvider>,
}

impl From<AppProviderState> for ProviderState {
    fn from(value: AppProviderState) -> ProviderState {
        ProviderState {
            vector: value.vector,
            embedding: value.embedding,
            document: value.document,
        }
    }
}

#[derive(Clone)]
pub struct ServiceState {
    pub document: DocumentService<PgPool>,
    pub vector: VectorService<PgPool>,
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
    pub default_chunkers: Vec<ChunkConfig>,
}

/// Creates and implements functions for `$target` to easily get an instance of whatever
/// the provider is for, i.e. `$provider_out`.
///
/// Additionally, creates a constant with the given feature literals so we can easily list them
/// to the client.
macro_rules! provider {
    (
        $( $target:ident -> $provider_out:ident ),+
    ) => {
        $(
            #[derive(Clone, Default)]
            pub struct $target {
                providers: HashMap<&'static str, Arc<dyn $provider_out + Send + Sync>>,
            }

            impl ProviderFactory<Arc<dyn $provider_out + Send + Sync>> for $target {
                /// AUTO-GENERATED BY THE `provider!` MACRO.
                /// SEE [crate::app::state] FOR MORE DETAILS.
                /// Obtain the provider for the given enum variant from the application state.
                fn get_provider(
                    &self,
                    input: &str,
                ) -> Result<Arc<dyn $provider_out + Send + Sync>, ChonkitError> {
                    match self.providers.get(input).cloned() {
                        Some(e) => Ok(e),
                        None => err!(InvalidProvider, "{input}"),
                    }
                }

                /// AUTO-GENERATED BY THE `provider!` MACRO.
                /// SEE [crate::app::state] FOR MORE DETAILS.
                /// A list of available providers for a given functionality.
                fn list_provider_ids(&self) -> Vec<&'static str> {
                    self.providers.keys().cloned().collect()
                }

                /// AUTO-GENERATED BY THE `provider!` MACRO.
                /// SEE [crate::app::state] FOR MORE DETAILS.
                /// Register a new provider in this factory.
                fn register(
                    &mut self,
                    id: &'static str,
                    provider: Arc<dyn $provider_out + Send + Sync>,
                ) {
                    self.providers.insert(id, provider);
                }
            }
        )+
    };
}

provider! {
    VectorDbProvider -> VectorDb,
    DocumentStoreProvider -> DocumentStore,
    EmbeddingProvider -> Embedder
}
