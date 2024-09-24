use chonkit::app::{batch::BatchEmbedderHandle, state::AppState};
use clap::Parser;
use tracing::info;

mod api;
mod dto;
mod router;

#[tokio::main]
async fn main() {
    let args = chonkit::config::StartArgs::parse();
    let state = chonkit::state(&args).await;
    let batch_embedder = chonkit::spawn_batch_embedder(state.clone());
    let addr = args.address();

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("error while starting TCP listener");

    let router = router::router(state, batch_embedder);

    info!("Listening on {addr}");

    axum::serve(listener, router)
        .await
        .expect("error while starting server");
}
