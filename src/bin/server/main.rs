use clap::Parser;
use tracing::info;

#[tokio::main]
async fn main() {
    let args = chonkit::config::StartArgs::parse();
    let app_state = chonkit::app::state::AppState::new(&args).await;
    let service_state = chonkit::app::state::ServiceState::from_app_state(&app_state);
    let batch_embedder = chonkit::app::state::spawn_batch_embedder(service_state.clone());

    let global_state = chonkit::app::state::GlobalState {
        app_state: app_state.clone(),
        service_state: service_state.clone(),
        batch_embedder: batch_embedder.clone(),
    };

    let addr = args.address();
    let origins = args.allowed_origins();

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("error while starting TCP listener");

    let router = chonkit::app::server::router::router(global_state, batch_embedder, origins);

    info!("Listening on {addr}");

    axum::serve(listener, router)
        .await
        .expect("error while starting server");
}
