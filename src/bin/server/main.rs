use clap::Parser;
use tracing::info;

#[tokio::main]
async fn main() {
    let args = chonkit::config::StartArgs::parse();
    let app = chonkit::app::state::AppState::new(&args).await;

    let addr = args.address();
    let origins = args.allowed_origins();

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("error while starting TCP listener");

    let router = chonkit::app::server::router::router(app, origins);

    info!("Listening on {addr}");

    axum::serve(listener, router)
        .await
        .expect("error while starting server");
}
