use std::time::Duration;

use crate::{
    document::{models::DirectoryEntry, DocumentData, DocumentMeta},
    error::LedgeknawError,
    llm::chunk::{ChunkConfig, Chunker, Recursive, SlidingWindow, SnappingWindow},
    state::DocumentService,
};
use axum::{
    http::Method,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use tower_http::{
    classify::ServerErrorsFailureClass,
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::{info, Span};

mod admin;

pub fn router(state: DocumentService) -> Router {
    let router = public_router(state.clone());

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST]);

    router
        .layer(TraceLayer::new_for_http().on_failure(
            |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                tracing::error!("{error}")
            },
        ))
        .layer(cors)
}

fn public_router(state: DocumentService) -> Router {
    Router::new()
        .nest_service(
            "/",
            ServeDir::new("dist").fallback(ServeFile::new("dist/index.html")),
        )
        .route("/meta/:id", get(document_meta))
        .route("/side", get(sidebar_init))
        .route("/side/:id", get(sidebar_entries))
        .route("/sync", get(sync))
        .route("/document/:id", get(document))
        .route("/document/:id/chunk", post(chunk))
        .with_state(state)
}

// DOCUMENT ROUTER

pub async fn document(
    state: axum::extract::State<DocumentService>,
    path: axum::extract::Path<String>,
) -> Result<Json<DocumentData>, LedgeknawError> {
    Ok(Json(state.read_file(&path.0).await?))
}

pub async fn document_meta(
    state: axum::extract::State<DocumentService>,
    id: axum::extract::Path<uuid::Uuid>,
) -> Result<Json<DocumentMeta>, LedgeknawError> {
    Ok(Json(state.get_file_meta(*id).await?))
}

pub async fn sidebar_init(
    state: axum::extract::State<DocumentService>,
) -> Result<Json<Vec<DirectoryEntry>>, LedgeknawError> {
    let docs = state.db.list_roots().await?;
    Ok(Json(docs))
}

pub async fn sidebar_entries(
    state: axum::extract::State<DocumentService>,
    path: axum::extract::Path<uuid::Uuid>,
) -> Result<Json<Vec<DirectoryEntry>>, LedgeknawError> {
    let files = state.db.list_entries(*path).await?;
    Ok(Json(files))
}

// STATE ROUTER

async fn sync(
    state: axum::extract::State<DocumentService>,
) -> Result<impl IntoResponse, LedgeknawError> {
    state.sync().await?;
    Ok(())
}

// CHUNK ROUTER

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ChunkPayload {
    SlidingWindow {
        config: ChunkConfig,
    },
    SnappingWindow {
        config: ChunkConfig,
        skip_f: Vec<String>,
        skip_b: Vec<String>,
    },
    Recursive {
        config: ChunkConfig,
        delimiters: Vec<String>,
    },
}

async fn chunk(
    service: axum::extract::State<DocumentService>,
    id: axum::extract::Path<String>,
    config: axum::extract::Json<ChunkPayload>,
) -> Result<impl IntoResponse, LedgeknawError> {
    let document = service.read_file(&id.0).await?;
    match config.0 {
        ChunkPayload::SlidingWindow {
            config: ChunkConfig { size, overlap },
        } => {
            info!("Chunking {} with SlidingWindow", id.0);
            let chunker = SlidingWindow::new(size, overlap);
            let chunks = chunker
                .chunk(&document.content)?
                .into_iter()
                .map(|chonk| String::from(chonk.content))
                .collect::<Vec<_>>();
            Ok(Json(chunks))
        }
        ChunkPayload::SnappingWindow {
            config: ChunkConfig { size, overlap },
            skip_f,
            skip_b,
        } => {
            info!("Chunking {} with SnappingWindow", id.0);
            let skip_f = skip_f.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            let skip_b = skip_b.iter().map(|s| s.as_str()).collect::<Vec<_>>();

            let chunker = SnappingWindow::new(size, overlap)
                .skip_forward(&skip_f)
                .skip_back(&skip_b);

            let chunks = chunker
                .chunk(&document.content)?
                .into_iter()
                .map(|chonk| String::from(chonk.content))
                .collect::<Vec<_>>();
            Ok(Json(chunks))
        }
        ChunkPayload::Recursive {
            config: ChunkConfig { size, overlap },
            delimiters,
        } => {
            let delims = delimiters.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            let chunker = Recursive::new(size, overlap, &delims);

            let chunks = chunker
                .chunk(&document.content)?
                .into_iter()
                .map(|chonk| String::from(chonk.content))
                .collect::<Vec<_>>();
            Ok(Json(chunks))
        }
    }
}
