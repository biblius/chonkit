use crate::app::service::ServiceState;
use tracing::info;

pub mod api;
pub mod dto;
pub mod router;

pub async fn server(addr: &str, services: ServiceState) {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("error while starting TCP listener");

    let router = router::router(services);

    info!("Listening on {addr}");

    axum::serve(listener, router)
        .await
        .expect("error while starting server");
}
