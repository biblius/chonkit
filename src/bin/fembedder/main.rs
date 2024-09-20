use axum::{
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chonkit::app::embedder::fastembed::FastEmbedder;
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use std::{sync::Arc, time::Duration};
use tower_http::{classify::ServerErrorsFailureClass, cors::CorsLayer, trace::TraceLayer};
use tracing::{info, Span};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let args = StartArgs::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let addr = &args.address;

    let fastembed = Arc::new(chonkit::app::embedder::fastembed::init());

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST]);

    let router = Router::new()
        .route("/_health", get(_health))
        .route("/embed", post(embed))
        .layer(cors)
        .layer(TraceLayer::new_for_http().on_failure(
            |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                tracing::error!("{error}")
            },
        ))
        .with_state(fastembed);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("error while starting TCP listener");

    info!("Listening on {addr}");

    axum::serve(listener, router)
        .await
        .expect("error while starting server");
}

#[derive(Debug, Parser)]
struct StartArgs {
    #[arg(short, long, default_value = "0.0.0.0:6969")]
    address: String,
}

#[derive(Debug, Deserialize)]
struct EmbedRequest {
    model: String,
    content: Vec<String>,
}

async fn embed(
    state: axum::extract::State<Arc<FastEmbedder>>,
    req: axum::extract::Json<EmbedRequest>,
) -> impl IntoResponse {
    let Some(model) = state.models.get(&req.model) else {
        let message = format!("Invalid model: {}", req.model);
        return (StatusCode::BAD_REQUEST, Json(json! { message }));
    };

    // Uses the default batch size of 256
    let embeddings = match model.embed(req.0.content, None) {
        Ok(e) => e,
        Err(e) => {
            let message = format!("Error while embedding: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json! { message }));
        }
    };

    (StatusCode::OK, Json(json! { embeddings }))
}

async fn _health() -> impl IntoResponse {
    "OK"
}
