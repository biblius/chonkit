use crate::{
    app::service::ServiceState,
    core::{
        chunk::Chunker,
        document::parser::ParseConfig,
        model::{document::DocumentType, Pagination},
        service::document::DocumentUpload,
    },
    ctrl::dto::{CreateCollectionPayload, EmbedPayload, SearchPayload, UploadResult},
    error::ChonkitError,
};
use axum::{
    extract::{DefaultBodyLimit, Query},
    http::Method,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use std::{collections::HashMap, time::Duration};
use tower_http::{classify::ServerErrorsFailureClass, cors::CorsLayer, trace::TraceLayer};
use tracing::{error, Span};
use validify::Validate;

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
        .route("/documents", get(list_documents))
        .route("/documents", post(upload_documents))
        .layer(DefaultBodyLimit::max(50_000_000))
        .route("/documents/:id", get(get_document))
        .route("/documents/:id", delete(delete_document))
        .route("/documents/:id/chunk/preview", post(chunk_preview))
        .route("/documents/:id/parse/preview", post(parse_preview))
        .route("/documents/sync", get(sync))
        .route("/vectors", get(list_collections))
        .route("/vectors", post(create_collection))
        .route("/vectors/models", get(list_embedding_models))
        .route("/embeddings/:id", post(embed))
        .route("/embeddings", post(search))
        .with_state(state)
}

// VECTOR ROUTER

async fn search(
    service: axum::extract::State<ServiceState>,
    search: axum::extract::Json<SearchPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let SearchPayload {
        ref model,
        ref query,
        ref collection,
        limit,
    } = search.0;

    let chunks = service
        .vector
        .search(model, query, collection, limit)
        .await?;

    Ok(Json(chunks))
}

async fn list_documents(
    service: axum::extract::State<ServiceState>,
    pagination: Option<axum::extract::Query<Pagination>>,
) -> Result<impl IntoResponse, ChonkitError> {
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;
    let documents = service.document.list_documents(pagination).await?;
    Ok(Json(documents))
}

async fn get_document(
    service: axum::extract::State<ServiceState>,
    id: axum::extract::Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ChonkitError> {
    let document = service.document.get_metadata(id.0).await?;
    Ok(Json(document))
}

async fn delete_document(
    service: axum::extract::State<ServiceState>,
    id: axum::extract::Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ChonkitError> {
    service.document.delete(id.0).await?;
    Ok(format!("Successfully deleted {}", id.0))
}

async fn upload_documents(
    service: axum::extract::State<ServiceState>,
    mut form: axum::extract::Multipart,
) -> Result<Json<UploadResult>, ChonkitError> {
    let mut documents = vec![];
    let mut errors = HashMap::new();

    while let Ok(Some(field)) = form.next_field().await {
        let Some(name) = field.file_name() else {
            continue;
        };

        let name = name.to_string();

        let file = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("error in form: {e}");
                errors.insert(name, e.to_string());
                continue;
            }
        };

        let typ = match DocumentType::try_from_file_name(&name) {
            Ok(ty) => ty,
            Err(e) => {
                error!("{e}");
                errors.insert(name, e.to_string());
                continue;
            }
        };

        let upload = DocumentUpload::new(name.to_string(), typ, &file);
        let document = service.document.upload(upload).await?;

        documents.push(document);
    }

    Ok(Json(UploadResult { documents, errors }))
}

async fn chunk_preview(
    service: axum::extract::State<ServiceState>,
    id: axum::extract::Path<uuid::Uuid>,
    chunker: axum::extract::Json<Chunker>,
) -> Result<impl IntoResponse, ChonkitError> {
    let parsed = service.document.chunk_preview(id.0, chunker.0).await?;
    Ok(Json(parsed))
}

async fn parse_preview(
    service: axum::extract::State<ServiceState>,
    id: axum::extract::Path<uuid::Uuid>,
    parser: axum::extract::Json<ParseConfig>,
) -> Result<impl IntoResponse, ChonkitError> {
    let parsed = service.document.parse_preview(id.0, parser.0).await?;
    Ok(Json(parsed))
}

async fn sync(
    service: axum::extract::State<ServiceState>,
) -> Result<impl IntoResponse, ChonkitError> {
    service.document.sync().await?;
    Ok("Successfully synced")
}

// VECTORS

async fn list_collections(
    service: axum::extract::State<ServiceState>,
    pagination: axum::extract::Query<Pagination>,
) -> Result<impl IntoResponse, ChonkitError> {
    let collections = service.vector.list_collections(pagination.0).await?;
    Ok(Json(collections))
}

async fn create_collection(
    service: axum::extract::State<ServiceState>,
    payload: axum::extract::Json<CreateCollectionPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let CreateCollectionPayload { name, model } = payload.0;
    service.vector.create_collection(&name, &model).await?;
    Ok("Successfully created collection")
}

async fn list_embedding_models(
    service: axum::extract::State<ServiceState>,
) -> Result<impl IntoResponse, ChonkitError> {
    let models = service
        .vector
        .list_embedding_models()
        .into_iter()
        .collect::<Vec<_>>();
    Ok(Json(models))
}

async fn embed(
    service: axum::extract::State<ServiceState>,
    id: axum::extract::Path<uuid::Uuid>,
    payload: axum::extract::Json<EmbedPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let document = service.document.get_metadata(id.0).await?;
    let content = service.document.get_content(id.0).await?;
    // service.vector.create_embeddings().await?;
    Ok("Successfully embedded")
}
