use std::time::Duration;

use crate::{
    document::{File, FileOrDir},
    dto::file::FileResponse,
    error::ChonkitError,
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
use uuid::Uuid;

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
        .route("/sync", get(sync))
        .route("/side", get(sidebar_init))
        .route("/side/:id", get(sidebar_entries))
        .route("/document/:id", get(get_file))
        .route("/document/:id/chunk", post(chunk))
        .with_state(state)
}

// DOCUMENT SERVICE ROUTER

pub async fn get_file(
    service: axum::extract::State<DocumentService>,
    id: axum::extract::Path<uuid::Uuid>,
) -> Result<Json<FileResponse>, ChonkitError> {
    let file = service.get_file(*id).await?;

    let FileOrDir::File(file) = file else {
        return Err(ChonkitError::NotFound(id.to_string()));
    };

    let content = service.get_file_contents(&file.path).await?;

    Ok(Json(FileResponse::from((file, content))))
}

async fn sync(
    service: axum::extract::State<DocumentService>,
) -> Result<impl IntoResponse, ChonkitError> {
    service.sync().await?;
    Ok(())
}

pub async fn sidebar_init(
    service: axum::extract::State<DocumentService>,
) -> Result<Json<Vec<File>>, ChonkitError> {
    let docs = service.db.list_root_files().await?;
    Ok(Json(docs))
}

pub async fn sidebar_entries(
    service: axum::extract::State<DocumentService>,
    id: axum::extract::Path<uuid::Uuid>,
) -> Result<Json<Vec<File>>, ChonkitError> {
    let files = service.db.list_children(*id).await?;
    Ok(Json(files))
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
    id: axum::extract::Path<Uuid>,
    config: axum::extract::Json<ChunkPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let file = service.get_file(id.0).await?;

    let FileOrDir::File(file) = file else {
        return Err(ChonkitError::NotFound(id.to_string()));
    };

    let content = service.get_file_contents(&file.path).await?;

    match config.0 {
        ChunkPayload::SlidingWindow {
            config: ChunkConfig { size, overlap },
        } => {
            info!("Chunking {} with SlidingWindow", id.0);
            let chunker = SlidingWindow::new(size, overlap);
            let chunks = chunker
                .chunk(&content)?
                .into_iter()
                .map(String::from)
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
                .chunk(&content)?
                .into_iter()
                .map(String::from)
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
                .chunk(&content)?
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>();
            Ok(Json(chunks))
        }
    }
}
