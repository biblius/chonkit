use crate::{
    config::{Config, StartArgs},
    document::db::DocumentDb,
    service::document::DocumentService,
};
use clap::Parser;
use qdrant_client::client::QdrantClient;
use std::num::NonZeroUsize;
use tracing::info;
use vector_db::VectorService;

pub const FILES_PER_THREAD: usize = 128;

lazy_static::lazy_static! {
    pub static ref MAX_THREADS: usize = std::thread::available_parallelism().unwrap_or(NonZeroUsize::new(1).unwrap()).into();
}

pub mod config;
pub mod db;
pub mod document;
pub mod dto;
pub mod error;
pub mod llm;
pub mod router;
pub mod service;
pub mod state;
pub mod vector_db;

#[tokio::main]
async fn main() {
    let StartArgs {
        config_path,
        address: host,
        port,
        log_level: level,
        qdrant_url,
        db_url,
    } = StartArgs::parse();

    tracing_subscriber::fmt().with_max_level(level).init();

    let db_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            info!("DATABASE_URL not set, falling back to {db_url}");
            db_url
        }
    };

    let qdrant_url = match std::env::var("QDRANT_URL") {
        Ok(url) => url,
        Err(_) => {
            info!("QDRANT_URL not set, falling back to {qdrant_url}");
            qdrant_url
        }
    };

    info!("Connecting to postgres at {db_url}");
    let db_pool = db::create_pool(&db_url).await;

    db::migrate(&db_pool).await;

    let addr = format!("{host}:{port}");

    let Config { directory, .. } = Config::read(config_path).expect("invalid config file");

    let document_db = DocumentDb::new(db_pool.clone()).await;

    let documents = DocumentService::new(document_db.clone());

    info!("Starting TCP listener on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("error while starting TCP listener");

    info!("Connecting to qdrant at {qdrant_url}");

    let qdrant = QdrantClient::from_url(&qdrant_url).build().unwrap();

    let vectorizer = VectorService::new(qdrant, db_pool);

    documents.sync(&[directory.as_path()]).await.unwrap();

    vectorizer.init().await.unwrap();

    let router = router::router(documents);

    //let mut hf_api = hf_hub::api::tokio::ApiBuilder::new()
    //    .with_progress(true)
    //    .with_token(Some(hf_token));

    //if let Some(cache_dir) = hf_cache {
    //    hf_api = hf_api.with_cache_dir(cache_dir);
    //}

    //let hf_api = hf_api.build().expect("could not build huggingface API");

    // vectorizer.test_vectors().await;

    axum::serve(listener, router)
        .await
        .expect("error while starting server");
}
