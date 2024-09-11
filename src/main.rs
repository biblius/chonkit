use crate::config::StartArgs;
use app::service::ServiceState;
use clap::Parser;
use pdfium_render::prelude::Pdfium;
use tracing_subscriber::EnvFilter;

pub mod app;
pub mod config;
pub mod core;
pub mod ctrl;
pub mod error;

pub const DEFAULT_COLLECTION_NAME: &str = "chonkit_default_0";

#[cfg(all(feature = "cli", feature = "http"))]
compile_error!("cannot run in both cli and http mode");

#[cfg(not(any(feature = "cli", feature = "http")))]
compile_error!("execution mode not set; run with `-F cli` or -F `http` to pick one");

#[cfg(not(any(feature = "qdrant", feature = "weaviate")))]
compile_error!("vector db provider not set; run with `-F qdrant` or -F `weaviate` to pick one");

#[cfg(all(feature = "qdrant", feature = "weaviate"))]
compile_error!("only one vector database provider is allowed");

#[tokio::main]
async fn main() {
    let args = StartArgs::parse();

    // Ensures the dynamic library is loaded and panics if it isn't
    Pdfium::default();

    let db_url = args.db_url();
    let vec_db_url = args.vec_db_url();
    let upload_path = args.upload_path();
    let log = args.log();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from(log))
        .init();

    let services = ServiceState::init(&db_url, &vec_db_url, &upload_path).await;

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
