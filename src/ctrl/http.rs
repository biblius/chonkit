use crate::app::service::ServiceState;

pub mod dto;
pub mod router;

pub async fn server(addr: &str, services: ServiceState) {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("error while starting TCP listener");

    let router = router::router(services);

    axum::serve(listener, router)
        .await
        .expect("error while starting server");
}
