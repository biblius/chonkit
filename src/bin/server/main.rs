use chonkit::app::service::ServiceState;
use clap::Parser;
use tracing::info;

mod api;
mod dto;
mod router;

#[tokio::main]
async fn main() {
    let args = chonkit::config::StartArgs::parse();
    let state = chonkit::state(&args).await;
    let addr = args.address();
    server(&addr, state).await;
}

async fn server(addr: &str, services: ServiceState) {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("error while starting TCP listener");

    let router = router::router(services);

    info!("Listening on {addr}");

    axum::serve(listener, router)
        .await
        .expect("error while starting server");
}
