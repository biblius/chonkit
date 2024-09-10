use crate::config::StartArgs;
use app::service::ServiceState;
use clap::Parser;
use tracing_subscriber::EnvFilter;

pub mod app;
pub mod config;
pub mod core;
pub mod ctrl;
pub mod error;

pub const DEFAULT_COLLECTION_NAME: &str = "chonkit_default_0";
pub const DEFAULT_COLLECTION_MODEL: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
pub const DEFAULT_COLLECTION_SIZE: usize = 384;

#[cfg(all(feature = "cli", feature = "http"))]
compile_error!("cannot run in both cli and http mode");

#[cfg(not(any(feature = "cli", feature = "http")))]
compile_error!("execution mode not set; run with `-F cli` or -F `http` to pick one");

#[cfg(not(any(feature = "qdrant", feature = "weaviate")))]
compile_error!("vector db provider not set; run with `-F qdrant` or -F `weaviate` to pick one");

#[cfg(all(feature = "qdrant", feature = "weaviate"))]
compile_error!("only one vector database provider is allowed");

#[cfg(feature = "http")]
async fn run() {
    run_server().await;
}

#[cfg(feature = "cli")]
async fn run() {
    run_cli().await;
}

#[tokio::main]
async fn main() {
    run().await;
}

#[cfg(feature = "http")]
async fn run_server() {
    let args = StartArgs::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from(args.log()))
        .init();

    let db_url = args.db_url();
    let vec_db_url = args.vec_db_url();

    let services = ServiceState::init(&db_url, &vec_db_url, &args.upload_path()).await;

    let addr = args.address();
    ctrl::http::server(&addr, services).await;
}

#[cfg(feature = "cli")]
async fn run_cli() {
    let args = StartArgs::parse();

    let db_url = args.db_url();
    let vec_db_url = args.vec_db_url();
    let upload_path = args.upload_path();
    let log = args.log();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from(log))
        .init();

    let services = ServiceState::init(&db_url, &vec_db_url, &upload_path).await;

    ctrl::cli::run(args.command, services).await;
}
