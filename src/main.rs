use crate::config::StartArgs;
use app::service::ServiceState;
use cfg_if::cfg_if;
use tracing_subscriber::EnvFilter;

pub mod app;
pub mod config;
pub mod core;
pub mod ctrl;
pub mod error;

pub const DEFAULT_UPLOAD_PATH: &str = "upload";
pub const TEST_DOCS_PATH: &str = "test/docs";

pub const DEFAULT_COLLECTION_NAME: &str = "chonkit_default_0";
pub const DEFAULT_COLLECTION_MODEL: &str = "Qdrant/all-MiniLM-L6-v2-onnx";
pub const DEFAULT_COLLECTION_SIZE: usize = 384;

#[cfg(all(feature = "cli", feature = "http"))]
compile_error!("cannot run with both cli and http enabled");

#[cfg(all(feature = "qdrant", feature = "weaviate"))]
compile_error!("only one vector database provider is allowed");

cfg_if!(
    if #[cfg(feature = "http")] {
        async fn run() { run_server().await; }
    } else if #[cfg(feature = "cli")]  {
        async fn run() { run_cli().await; }
    }
);

#[tokio::main]
async fn main() {
    run().await;
}

#[cfg(feature = "http")]
async fn run_server() {
    let args = <StartArgs as clap::Parser>::parse();

    tracing_subscriber::fmt()
        .with_max_level(args.log_level)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let db_url = args.db_url();
    let vec_db_url = args.vec_db_url();

    let services = ServiceState::init(
        &db_url,
        &vec_db_url,
        &args.upload_path.unwrap_or(DEFAULT_UPLOAD_PATH.to_string()),
    )
    .await;

    let addr = format!("{}:{}", args.address, args.port);
    ctrl::http::server(&addr, services).await;
}

#[cfg(feature = "cli")]
async fn run_cli() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(
            "debug,sqlx=off,h2=off,lopdf=off,chonkit=trace",
        ))
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let vec_db_url = std::env::var("VEC_DATABASE_URL").expect("VEC_DATABASE_URL not set");
    let upload_path = std::env::var("UPLOAD_PATH").unwrap_or(DEFAULT_UPLOAD_PATH.to_string());

    let services = ServiceState::init(&db_url, &vec_db_url, &upload_path).await;

    ctrl::cli::run(services).await;
}
