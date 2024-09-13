use crate::config::StartArgs;
use app::{
    document::store::FsDocumentStore, embedder::fastembed::FastEmbedder, service::ServiceState,
};
use clap::Parser;
use pdfium_render::prelude::Pdfium;
use std::sync::Arc;
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

    let postgres = crate::app::repo::pg::init(&db_url).await;

    let fastembed = Arc::new(FastEmbedder);
    let fs_store = Arc::new(FsDocumentStore::new(&upload_path));

    #[cfg(feature = "qdrant")]
    let qdrant = Arc::new(crate::app::vector::qdrant::init(&vec_db_url));

    #[cfg(feature = "weaviate")]
    let weaviate = Arc::new(crate::app::vector::weaviate::init(&vec_db_url));

    let services = ServiceState {
        postgres,
        fs_store,
        fastembed,

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
