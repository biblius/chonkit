use app::{
    batch::{BatchEmbedder, BatchEmbedderHandle},
    service::AppState,
};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

pub mod app;
pub mod cli;
pub mod config;
pub mod core;
pub mod error;

pub const DEFAULT_COLLECTION_NAME: &str = "chonkit_default_0";

pub async fn state(args: &config::StartArgs) -> AppState {
    // Ensures the dynamic library is loaded and panics if it isn't
    pdfium_render::prelude::Pdfium::default();

    #[cfg(all(
        feature = "fembed",
        not(any(feature = "fe-local", feature = "fe-remote"))
    ))]
    compile_error!("either `fe-local` or `fe-remote` must be enabled when running with `fembed`");

    let db_url = args.db_url();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from(args.log()))
        .init();

    let postgres = app::repo::pg::init(&db_url).await;

    let fs_store = Arc::new(app::document::store::FsDocumentStore::new(
        &args.upload_path(),
    ));

    #[cfg(feature = "fe-local")]
    tracing::info!(
        "Cuda available: {:?}",
        ort::ExecutionProvider::is_available(&ort::CUDAExecutionProvider::default())
    );

    #[cfg(feature = "fe-local")]
    let fastembed = Arc::new(crate::app::embedder::fastembed::init());

    #[cfg(feature = "fe-remote")]
    let fastembed = Arc::new(crate::app::embedder::fastembed::init(args.fembed_url()));

    #[cfg(feature = "openai")]
    let openai = Arc::new(crate::app::embedder::openai::OpenAiEmbeddings::new(
        &args.open_ai_key(),
    ));

    #[cfg(feature = "qdrant")]
    let qdrant = Arc::new(crate::app::vector::qdrant::init(&args.qdrant_url()));

    #[cfg(feature = "weaviate")]
    let weaviate = Arc::new(crate::app::vector::weaviate::init(&args.weaviate_url()));

    AppState {
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

pub fn spawn_batch_embedder(state: AppState) -> BatchEmbedderHandle {
    let (tx, rx) = tokio::sync::mpsc::channel(128);
    BatchEmbedder::new(rx, state).start();
    tx
}
