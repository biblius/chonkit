use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chonkit_embedders::fastembed::local::LocalFastEmbedder as FastEmbedder;
use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration};
use tower_http::{classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::{info, Span};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let args = StartArgs::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_str("debug").expect("invalid logging config"))
        .init();

    let addr = &args.address;

    let fastembed = Arc::new(FastEmbedder::new());

    let router = Router::new()
        .route("/_health", get(_health))
        .route("/embed", post(embed))
        .route("/list", get(list_embedding_models))
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

// Routes

async fn embed(
    state: State<Arc<FastEmbedder>>,
    Json(EmbedRequest { ref model, content }): axum::extract::Json<EmbedRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Uses the default batch size of 256
    let content = content.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    match state.embed(&content, model) {
        Ok(embeddings) => (StatusCode::OK, Json(json! {embeddings})),
        Err(e) => {
            tracing::error!("{e}");
            let error = e.to_string();
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json! { error }))
        }
    }
}

async fn list_embedding_models(
    state: axum::extract::State<Arc<FastEmbedder>>,
) -> (StatusCode, Json<serde_json::Value>) {
    let models = state
        .list_models()
        .into_iter()
        .map(|model| (model.model_code, model.dim))
        .collect::<HashMap<String, usize>>();

    (StatusCode::OK, Json(json! { models }))
}

async fn _health() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

// DTO

#[derive(Debug, Deserialize)]
pub struct EmbedRequest {
    model: String,
    content: Vec<String>,
}
