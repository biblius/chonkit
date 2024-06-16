use crate::{
    config::{Config, StartArgs},
    document::db::DocumentDb,
    state::DocumentService,
};
use clap::Parser;
use config::HfConfig;
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
pub mod state;
pub mod vector_db;

#[tokio::main]
async fn main() {
    let StartArgs {
        config_path,
        address: host,
        port,
        log_level: level,
    } = StartArgs::parse();

    tracing_subscriber::fmt().with_max_level(level).init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let db_pool = db::create_pool(&db_url).await;

    db::migrate(&db_pool).await;

    let addr = format!("{host}:{port}");

    let Config { directory, .. } = Config::read(config_path).expect("invalid config file");

    let document_db = DocumentDb::new(db_pool.clone()).await;

    let documents = DocumentService::new(document_db.clone());
    documents.sync().await.expect("unable to sync");

    info!("Now listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("error while starting TCP listener");

    let qdrant = QdrantClient::from_url("http://localhost:6334")
        .build()
        .unwrap();

    let vectorizer = VectorService::new(qdrant, db_pool);

    documents.init(&[directory.as_path()]).await.unwrap();
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
