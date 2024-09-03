use crate::config::StartArgs;
use app::service::ServiceState;
use cfg_if::cfg_if;
use qdrant_client::Qdrant;
use tracing_subscriber::EnvFilter;

pub mod app;
pub mod config;
pub mod control;
pub mod core;
pub mod error;

pub const DB_URL: &str = "postgresql://postgres:postgres@localhost:5433/chonkit";
pub const VEC_DB_URL: &str = "http://localhost:6334";
pub const DEFAULT_COLLECTION_NAME: &str = "__default__";
pub const DEFAULT_COLLECTION_MODEL: &str = "Qdrant/all-MiniLM-L6-v2-onnx";

pub const DEFAULT_LOG: &str = "info,h2=off,lopdf=off,chonkit=debug";

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
    let qd_url = args.qdrant_url();

    let db_pool = app::repo::pg::init(&db_url).await;
    let qdrant = Qdrant::from_url(&qd_url).build().unwrap();

    let services = ServiceState::init(db_pool, qdrant).await;

    let addr = format!("{}:{}", args.address, args.port);
    control::http::server(&addr, services).await;
}

#[cfg(feature = "cli")]
async fn run_cli() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(
            "debug,sqlx=off,h2=off,lopdf=off,chonkit=debug",
        ))
        .init();

    let db_pool = app::repo::pg::init(DB_URL).await;
    let qdrant = Qdrant::from_url(VEC_DB_URL).build().unwrap();

    let services = ServiceState::init(db_pool, qdrant).await;
    control::cli::run(services).await;
}
