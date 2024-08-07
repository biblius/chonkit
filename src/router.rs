use std::time::Duration;

use crate::{
    dto::file::FileResponse,
    error::ChonkitError,
    model::document::FileOrDir,
    service::{chunk::ChunkInput, ServiceState},
};
use axum::{
    http::Method,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use tower_http::{classify::ServerErrorsFailureClass, cors::CorsLayer, trace::TraceLayer};
use tracing::Span;
use uuid::Uuid;

pub fn router(state: ServiceState) -> Router {
    let router = public_router(state.clone());

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_methods([Method::GET, Method::POST]);

    router
        .layer(TraceLayer::new_for_http().on_failure(
            |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                tracing::error!("{error}")
            },
        ))
        .layer(cors)
}

fn public_router(state: ServiceState) -> Router {
    Router::new()
        .route("/sync", get(sync))
        .route("/files", get(sidebar_init))
        .route("/files/:id", get(sidebar_entries))
        .route("/documents/:id", get(get_file))
        .route("/documents/:id/chunk", post(chunk))
        .route("/embeddings/", post(embed))
        .route("/embeddings/models", get(list_embedding_models))
        .route("/embeddings/collections", get(list_collections))
        .route("/embeddings/collections", post(create_collection))
        .route("/embeddings/search", post(search))
        .with_state(state)
}

// DOCUMENT SERVICE ROUTER

pub async fn get_file(
    service: axum::extract::State<ServiceState>,
    id: axum::extract::Path<uuid::Uuid>,
) -> Result<Json<FileResponse>, ChonkitError> {
    let file = service.document.get_file(*id).await?;

    let FileOrDir::File(file) = file else {
        return Err(ChonkitError::NotFound(id.to_string()));
    };

    let content = service.document.get_file_contents(&file.path).await?;

    Ok(Json(FileResponse::from((file, content))))
}

async fn sync(
    service: axum::extract::State<ServiceState>,
) -> Result<impl IntoResponse, ChonkitError> {
    service.document.trim_non_existent().await?;
    Ok(())
}

pub async fn sidebar_init(
    service: axum::extract::State<ServiceState>,
) -> Result<impl IntoResponse, ChonkitError> {
    let docs = service.document.list_root_files().await?;
    Ok(Json(docs))
}

pub async fn sidebar_entries(
    service: axum::extract::State<ServiceState>,
    id: axum::extract::Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ChonkitError> {
    let files = service.document.list_children(*id).await?;
    Ok(Json(files))
}

// CHUNK ROUTER

async fn chunk(
    service: axum::extract::State<ServiceState>,
    id: axum::extract::Path<Uuid>,
    config: axum::extract::Json<ChunkInput>,
) -> Result<impl IntoResponse, ChonkitError> {
    let file = service.document.get_file(id.0).await?;

    let FileOrDir::File(file) = file else {
        return Err(ChonkitError::NotFound(id.to_string()));
    };

    let content = service.document.get_file_contents(&file.path).await?;
    let chunks = service
        .chunk
        .chunk(config.0, &file, &content)
        .unwrap()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    Ok(Json(chunks))
}

// VECTOR ROUTER

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmbedPayload {
    /// Document ID.
    id: uuid::Uuid,
    /// Vectpr collection
    collection: String,
    /// Chunking config.
    input: ChunkInput,
}

async fn embed(
    service: axum::extract::State<ServiceState>,
    config: axum::extract::Json<EmbedPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let EmbedPayload {
        id,
        collection,
        input,
    } = config.0;

    let file = service.document.get_file(id).await?;

    let FileOrDir::File(file) = file else {
        return Err(ChonkitError::NotFound(id.to_string()));
    };

    let content = service.document.get_file_contents(&file.path).await?;
    let chunks = service.chunk.chunk(input, &file, &content)?;

    service
        .vector
        .embed(
            chunks,
            fastembed::EmbeddingModel::AllMiniLML6V2,
            &collection,
        )
        .await;

    Ok(format!("Successfully embedded {}", file.name))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchPayload {
    model: String,
    query: String,
    collection: String,
}

async fn search(
    service: axum::extract::State<ServiceState>,
    search: axum::extract::Json<SearchPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let SearchPayload {
        ref model,
        ref query,
        collection,
    } = search.0;

    let model = service.vector.model_for_str(model).ok_or_else(|| {
        ChonkitError::UnsupportedEmbeddingModel(format!("{model} is not a valid embedding model",))
    })?;

    let chunks = service.vector.search(model.model, query, collection).await;

    Ok(Json(chunks))
}

async fn list_collections(
    service: axum::extract::State<ServiceState>,
) -> Result<impl IntoResponse, ChonkitError> {
    let collections = service.vector.list_collections().await?;
    Ok(Json(collections))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCollectionPayload {
    name: String,
    model: String,
}

async fn create_collection(
    service: axum::extract::State<ServiceState>,
    payload: axum::extract::Json<CreateCollectionPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let CreateCollectionPayload { name, model } = payload.0;

    let model = service.vector.model_for_str(&model).ok_or_else(|| {
        ChonkitError::UnsupportedEmbeddingModel(format!("{model} is not a valid embedding model",))
    })?;

    service.vector.create_collection(&name, model.model).await?;

    Ok("Successfully created collection")
}

async fn list_embedding_models(
    service: axum::extract::State<ServiceState>,
) -> Result<impl IntoResponse, ChonkitError> {
    let models = service
        .vector
        .list_embedding_models()
        .into_iter()
        .map(|info| info.model.to_string())
        .collect::<Vec<_>>();
    Ok(Json(models))
}
