use axum::{
    extract::{Query, State},
    http::{Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chonkit::core::embedder::Embedder;
use chonkit::{app::embedder::fastembed::FastEmbedder, error::ChonkitError};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, sync::Arc, time::Duration};
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

    let fastembed = Arc::new(FastEmbedder::new());

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::PUT,
            Method::PATCH,
        ]);

    let router = Router::new()
        .route("/_health", get(_health))
        .route("/embed", post(embed))
        .route("/list", get(list_embedding_models))
        .route("/default", get(default_model))
        .route("/size", get(size))
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

// Routes

async fn embed(
    state: State<Arc<FastEmbedder>>,
    Json(EmbedRequest { ref model, content }): axum::extract::Json<EmbedRequest>,
) -> Result<impl IntoResponse, ChonkitError> {
    // Uses the default batch size of 256
    let content = content.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let embeddings = state.embed(&content, model).await?;

    Ok(Json(EmbedResponse { embeddings }))
}

async fn list_embedding_models(
    state: axum::extract::State<Arc<FastEmbedder>>,
) -> impl IntoResponse {
    let models = state
        .list_embedding_models()
        .into_iter()
        .collect::<HashMap<String, usize>>();

    (StatusCode::OK, Json(json! { models }))
}

async fn default_model(state: State<Arc<FastEmbedder>>) -> impl IntoResponse {
    let (model, size) = state.default_model();
    Json(DefaultModelResponse { model, size })
}

async fn size(
    state: State<Arc<FastEmbedder>>,
    Query(req): Query<SizeRequest>,
) -> Result<impl IntoResponse, ChonkitError> {
    let size = state
        .size(&req.model)
        .ok_or_else(|| ChonkitError::InvalidEmbeddingModel(req.model))?;
    Ok(Json(SizeResponse { size }))
}

async fn _health() -> impl IntoResponse {
    "OK"
}

// DTO

#[derive(Debug, Deserialize)]
pub struct EmbedRequest {
    model: String,
    content: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SizeRequest {
    model: String,
}

#[derive(Debug, Serialize)]
pub struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Serialize)]
pub struct DefaultModelResponse {
    model: String,
    size: usize,
}

#[derive(Debug, Serialize)]
pub struct SizeResponse {
    size: usize,
}
