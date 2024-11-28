/// Concrete implementations of the [core] module.
pub mod app;

/// Application starting arguments and configuration.
pub mod config;

/// Core business logic.
pub mod core;

/// Error types.
pub mod error;

use clap::Parser;
use tracing::info;

#[tokio::main]
async fn main() {
    let args = crate::config::StartArgs::parse();
    let app = crate::app::state::AppState::new(&args).await;

    let addr = args.address();
    let origins = args.allowed_origins();

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("error while starting TCP listener");

    let router = crate::app::server::router::router(app, origins);

    info!("Listening on {addr}");

    axum::serve(listener, router)
        .await
        .expect("error while starting server");
}
