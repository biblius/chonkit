use super::api::ApiDoc;
use crate::{
    app::service::ServiceState,
    core::{
        chunk::Chunker,
        document::parser::ParseConfig,
        model::{document::DocumentType, Pagination},
        service::{
            document::dto::{ChunkPreviewPayload, DocumentUpload},
            vector::dto::{CreateCollection, CreateEmbeddings, SearchPayload},
        },
    },
    ctrl::http::dto::UploadResult,
    error::ChonkitError,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::Method,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use std::{collections::HashMap, time::Duration};
use tower_http::{classify::ServerErrorsFailureClass, cors::CorsLayer, trace::TraceLayer};
use tracing::{error, Span};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;
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
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
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
        .route("/documents/:id/chunk", put(update_chunk_config))
        .route("/documents/:id/parse/preview", post(parse_preview))
        .route("/documents/:id/parse", put(update_parse_config))
        .route("/documents/sync", get(sync))
        .route("/vectors/collections", get(list_collections))
        .route("/vectors/collections", post(create_collection))
        .route("/vectors/collections/:id", get(get_collection))
        .route("/vectors/models", get(list_embedding_models))
        .route("/vectors/collections/:id/embed/:doc_id", post(embed))
        .route("/vectors/search", post(search))
        .with_state(state)
}

#[utoipa::path(
    get,
    path = "/documents",
    responses(
        (status = 200, description = "List documents", body = [Document]),
        (status = 400, description = "Invalid pagination parameters"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("pagination" = Pagination, Query, description = "Pagination parameters")
    ),
)]
async fn list_documents(
    service: State<ServiceState>,
    pagination: Option<Query<Pagination>>,
) -> Result<impl IntoResponse, ChonkitError> {
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;
    let documents = service.document.list_documents(pagination).await?;
    Ok(Json(documents))
}

#[utoipa::path(
    get,
    path = "/documents/{id}",
    responses(
        (status = 200, description = "Get document by id", body = Document),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID")
    )
)]
async fn get_document(
    service: axum::extract::State<ServiceState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ChonkitError> {
    let document = service.document.get_config(id).await?;
    Ok(Json(document))
}

#[utoipa::path(
    delete,
    path = "/documents/{id}",
    responses(
        (status = 200, description = "Delete document by id"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID")
    )
)]
async fn delete_document(
    service: axum::extract::State<ServiceState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ChonkitError> {
    service.document.delete(id).await?;
    Ok(format!("Successfully deleted {id}"))
}

#[utoipa::path(
    post,
    path = "/documents",
    responses(
        (status = 200, description = "Upload documents", body = UploadResult),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    )
)]
async fn upload_documents(
    service: axum::extract::State<ServiceState>,
    mut form: axum::extract::Multipart,
) -> Result<Json<UploadResult>, ChonkitError> {
    let mut documents = vec![];
    let mut errors = HashMap::<String, Vec<String>>::new();

    while let Ok(Some(field)) = form.next_field().await {
        let Some(name) = field.file_name() else {
            continue;
        };

        let name = name.to_string();

        let file = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("error in form: {e}");
                errors
                    .entry(name)
                    .and_modify(|entry| entry.push(e.to_string()))
                    .or_insert_with(|| vec![e.to_string()]);
                continue;
            }
        };

        let typ = match DocumentType::try_from_file_name(&name) {
            Ok(ty) => ty,
            Err(e) => {
                error!("{e}");
                errors
                    .entry(name)
                    .and_modify(|entry| entry.push(e.to_string()))
                    .or_insert_with(|| vec![e.to_string()]);
                continue;
            }
        };

        let upload = DocumentUpload::new(name.to_string(), typ, &file);
        let document = service.document.upload(upload).await?;

        documents.push(document);
    }

    Ok(Json(UploadResult { documents, errors }))
}

#[utoipa::path(
    put,
    path = "/documents/{id}/chunk",
    responses(
        (status = 200, description = "Update chunk configuration"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID"),
    ),
    request_body = Chunker
)]
async fn update_chunk_config(
    service: State<ServiceState>,
    Path(document_id): Path<uuid::Uuid>,
    Json(chunker): Json<Chunker>,
) -> Result<impl IntoResponse, ChonkitError> {
    service
        .document
        .update_chunker(document_id, chunker)
        .await?;
    Ok(format!("Successfully updated chunker for {document_id}"))
}

#[utoipa::path(
    post,
    path = "/documents/{id}/chunk/preview",
    responses(
        (status = 200, description = "Preview chunk parsing", body = ChunkPreviewPayload),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID"),
    ),
    request_body = ChunkPreviewPayload
)]
async fn chunk_preview(
    service: State<ServiceState>,
    Path(id): Path<uuid::Uuid>,
    config: Option<Json<ChunkPreviewPayload>>,
) -> Result<impl IntoResponse, ChonkitError> {
    let parsed = service
        .document
        .chunk_preview(id, config.map(|c| c.0))
        .await?;
    Ok(Json(parsed))
}

#[utoipa::path(
    put,
    path = "/documents/{id}/parse",
    responses(
        (status = 200, description = "Update parse configuration"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID"),
    ),
    request_body = ParseConfig
)]
async fn update_parse_config(
    service: State<ServiceState>,
    Path(document_id): Path<uuid::Uuid>,
    Json(config): Json<ParseConfig>,
) -> Result<impl IntoResponse, ChonkitError> {
    service.document.update_parser(document_id, config).await?;
    Ok(format!("Successfully updated parser for {document_id}"))
}

#[utoipa::path(
    post,
    path = "/documents/{id}/parse/preview",
    responses(
        (status = 200, description = "Preview document parse result"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body(content = Option<ParseConfig>, description = "Optional parse configuration for preview")
)]
async fn parse_preview(
    service: State<ServiceState>,
    Path(id): Path<uuid::Uuid>,
    parser: Option<Json<ParseConfig>>,
) -> Result<impl IntoResponse, ChonkitError> {
    let parsed = service
        .document
        .parse_preview(id, parser.map(|c| c.0))
        .await?;
    Ok(Json(parsed))
}

#[utoipa::path(
    get,
    path = "/documents/sync", 
    responses(
        (status = 200, description = "Successfully synced"),
        (status = 500, description = "Internal server error")
    )
)]
async fn sync(
    service: axum::extract::State<ServiceState>,
) -> Result<impl IntoResponse, ChonkitError> {
    service.document.sync().await?;
    Ok("Successfully synced")
}

// VECTORS

#[utoipa::path(
    get,
    path = "/vectors/collections", 
    responses(
        (status = 200, description = "List collections"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("pagination" = Pagination, Query, description = "Pagination parameters")
    )
)]
async fn list_collections(
    service: State<ServiceState>,
    Query(p): Query<Pagination>,
) -> Result<impl IntoResponse, ChonkitError> {
    let collections = service.vector.list_collections(p).await?;
    Ok(Json(collections))
}

#[utoipa::path(
    post,
    path = "/vectors/collections", 
    responses(
        (status = 200, description = "Collection created successfully"),
        (status = 500, description = "Internal server error")
    ),
    request_body = CreateCollection
)]
async fn create_collection(
    service: State<ServiceState>,
    Json(payload): Json<CreateCollection>,
) -> Result<impl IntoResponse, ChonkitError> {
    let collection = service.vector.create_collection(payload).await?;
    Ok(Json(collection))
}

#[utoipa::path(
    get,
    path = "/vectors/collections/{id}", 
    responses(
        (status = 200, description = "Collection retrieved successfully"),
        (status = 404, description = "Collection not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Collection ID") // Adjusted param name
    )
)]
async fn get_collection(
    service: State<ServiceState>,
    Path(collection_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ChonkitError> {
    let collection = service.vector.get_collection(collection_id).await?;
    Ok(Json(collection))
}

#[utoipa::path(
    get,
    path = "/vectors/models", 
    responses(
        (status = 200, description = "List available embedding models"),
        (status = 500, description = "Internal server error")
    )
)]
async fn list_embedding_models(
    service: State<ServiceState>,
) -> Result<impl IntoResponse, ChonkitError> {
    let models = service
        .vector
        .list_embedding_models()
        .into_iter()
        .collect::<HashMap<String, usize>>();
    Ok(Json(models))
}

#[utoipa::path(
    post,
    path = "/vectors/collections/{id}/embed/{doc_id}", 
    responses(
        (status = 200, description = "Embeddings created successfully"),
        (status = 404, description = "Collection or document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Collection ID"),
        ("doc_id" = Uuid, Path, description = "Document ID")
    )
)]
async fn embed(
    service: axum::extract::State<ServiceState>,
    Path((collection_id, document_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ChonkitError> {
    let collection = service.vector.get_collection(collection_id).await?;

    let content = service.document.get_content(document_id).await?;
    let chunks = service.document.get_chunks(document_id, &content).await?;

    let embeddings = CreateEmbeddings {
        id: document_id,
        collection: collection.id,
        chunks,
    };

    service.vector.create_embeddings(embeddings).await?;

    Ok("Successfully created embeddings")
}

#[utoipa::path(
    post,
    path = "/vectors/search", // Adjusted path
    responses(
        (status = 200, description = "Search results returned"),
        (status = 500, description = "Internal server error")
    ),
    request_body = SearchPayload
)]
async fn search(
    service: State<ServiceState>,
    Json(search): Json<SearchPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let chunks = service.vector.search(search).await?;
    Ok(Json(chunks))
}
