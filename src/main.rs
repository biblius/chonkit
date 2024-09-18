use pdfium_render::prelude::Pdfium;

pub mod app;
pub mod config;
pub mod core;
pub mod ctrl;
pub mod error;

pub const DEFAULT_COLLECTION_NAME: &str = "chonkit_default_0";

#[tokio::main]
async fn main() {
    #[cfg(all(feature = "cli", feature = "http"))]
    compile_error!("cannot run in both cli and http mode");

    #[cfg(not(any(test, feature = "cli", feature = "http")))]
    compile_error!("execution mode not set; run with `-F cli` or -F `http` to pick one");

    // Ensures the dynamic library is loaded and panics if it isn't
    Pdfium::default();

    #[cfg(any(feature = "cli", feature = "http"))]
    run().await;
}

#[cfg(any(feature = "cli", feature = "http"))]
async fn run() {
    use crate::config::StartArgs;
    use app::{document::store::FsDocumentStore, service::ServiceState};
    use clap::Parser;
    use std::sync::Arc;
    use tracing::info;
    use tracing_subscriber::EnvFilter;

    let args = StartArgs::parse();

    let db_url = args.db_url();
    let upload_path = args.upload_path();
    let log = args.log();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from(log))
        .init();

    #[cfg(feature = "fembed")]
    info!(
        "Cuda available: {:?}",
        ort::ExecutionProvider::is_available(&ort::CUDAExecutionProvider::default())
    );

    let postgres = crate::app::repo::pg::init(&db_url).await;

    let fs_store = Arc::new(FsDocumentStore::new(&upload_path));

    #[cfg(feature = "fembed")]
    let fastembed = Arc::new(crate::app::embedder::fastembed::init());

    #[cfg(feature = "openai")]
    let openai = Arc::new(crate::app::embedder::openai::OpenAiEmbeddings::new(
        &args.open_ai_key(),
    ));

    #[cfg(feature = "qdrant")]
    let qdrant = Arc::new(crate::app::vector::qdrant::init(&args.qdrant_url()));

    #[cfg(feature = "weaviate")]
    let weaviate = Arc::new(crate::app::vector::weaviate::init(&args.weaviate_url()));

    let services = ServiceState {
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
    };

    #[cfg(feature = "http")]
    {
        let addr = args.address();
        ctrl::http::server(&addr, services).await;
    }

    #[cfg(feature = "cli")]
    {
        ctrl::cli::run(args.command, services).await;
    }
}
