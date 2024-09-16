use super::{
    api::ApiDoc,
    dto::{CreateCollectionPayload, SearchPayload},
};
use crate::{
    app::service::ServiceState,
    core::{
        chunk::Chunker,
        document::parser::ParseConfig,
        model::{document::DocumentType, Pagination},
        service::{
            document::{
                dto::{ChunkPreviewPayload, DocumentUpload},
                DocumentService,
            },
            vector::{
                dto::{CreateEmbeddings, Search},
                VectorService,
            },
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
use validify::{Validate, Validify};

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
        .route("/documents/sync/:provider", get(sync))
        .route("/vectors/collections", get(list_collections))
        .route("/vectors/collections", post(create_collection))
        .route("/vectors/collections/:id", get(get_collection))
        .route(
            "/vectors/embeddings/:provider/models",
            get(list_embedding_models),
        )
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
    state: State<ServiceState>,
    pagination: Option<Query<Pagination>>,
    src: Option<Query<String>>,
) -> Result<impl IntoResponse, ChonkitError> {
    let Query(pagination) = pagination.unwrap_or_default();
    pagination.validate()?;
    let service = DocumentService::new(state.postgres.clone());
    let documents = service
        .list_documents(pagination, src.map(|s| s.0).as_deref())
        .await?;
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
    state: axum::extract::State<ServiceState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = DocumentService::new(state.postgres.clone());
    let document = service.get_config(id).await?;
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
    state: axum::extract::State<ServiceState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = DocumentService::new(state.postgres.clone());
    let document = service.get_document(id).await?;
    let store = state.store(document.src.try_into()?);
    service.delete(&*store, id).await?;
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
    state: axum::extract::State<ServiceState>,
    Query(provider): Query<String>,
    mut form: axum::extract::Multipart,
) -> Result<Json<UploadResult>, ChonkitError> {
    let mut documents = vec![];
    let mut errors = HashMap::<String, Vec<String>>::new();

    let service = DocumentService::new(state.postgres.clone());
    let store = state.store(provider.try_into()?);

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
        let document = service.upload(&*store, upload).await?;

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
    state: State<ServiceState>,
    Path(document_id): Path<uuid::Uuid>,
    Json(chunker): Json<Chunker>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = DocumentService::new(state.postgres.clone());
    service.update_chunker(document_id, chunker).await?;
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
    state: State<ServiceState>,
    Path(id): Path<uuid::Uuid>,
    config: Option<Json<ChunkPreviewPayload>>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = DocumentService::new(state.postgres.clone());
    let document = service.get_document(id).await?;
    let store = state.store(document.src.as_str().try_into()?);
    let parsed = service
        .chunk_preview(&*store, &document, config.map(|c| c.0).unwrap_or_default())
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
    state: State<ServiceState>,
    Path(document_id): Path<uuid::Uuid>,
    Json(config): Json<ParseConfig>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = DocumentService::new(state.postgres.clone());
    service.update_parser(document_id, config).await?;
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
    state: State<ServiceState>,
    Path(id): Path<uuid::Uuid>,
    parser: Option<Json<ParseConfig>>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = DocumentService::new(state.postgres.clone());
    let document = service.get_document(id).await?;
    let store = state.store(document.src.try_into()?);
    let parsed = service
        .parse_preview(&*store, id, parser.map(|c| c.0))
        .await?;
    Ok(Json(parsed))
}

#[utoipa::path(
    get,
    path = "/documents/sync/{provider}", 
    responses(
        (status = 200, description = "Successfully synced"),
        (status = 500, description = "Internal server error")
    )
)]
async fn sync(
    state: axum::extract::State<ServiceState>,
    Path(provider): Path<String>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = DocumentService::new(state.postgres.clone());
    let store = state.store(provider.try_into()?);
    service.sync(&*store).await?;
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
    state: State<ServiceState>,
    Query(p): Query<Pagination>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let collections = service.list_collections(p).await?;
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
    state: State<ServiceState>,
    Json(payload): Json<CreateCollectionPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let vector_db = state.vector_db(payload.vector_provider.as_str().try_into()?);
    let embedder = state.embedder(payload.embedding_provider.as_str().try_into()?);
    let collection = service
        .create_collection(&*vector_db, &*embedder, payload.into())
        .await?;
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
    state: State<ServiceState>,
    Path(collection_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let collection = service.get_collection(collection_id).await?;
    Ok(Json(collection))
}

#[utoipa::path(
    get,
    path = "/vectors/embeddings/{provider}/models", 
    responses(
        (status = 200, description = "List available embedding models"),
        (status = 500, description = "Internal server error")
    )
)]
async fn list_embedding_models(
    state: State<ServiceState>,
    Path(provider): Path<String>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let embedder = state.embedder(provider.as_str().try_into()?);
    let models = service
        .list_embedding_models(&*embedder)
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
    state: axum::extract::State<ServiceState>,
    Path((collection_id, document_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ChonkitError> {
    let d_service = DocumentService::new(state.postgres.clone());
    let v_service = VectorService::new(state.postgres.clone());

    let document = d_service.get_document(document_id).await?;
    let collection = v_service.get_collection(collection_id).await?;

    let store = state.store(document.src.as_str().try_into()?);
    let vector_db = state.vector_db(collection.provider.as_str().try_into()?);
    let embedder = state.embedder(collection.embedder.as_str().try_into()?);

    let content = d_service.get_content(&*store, document_id).await?;
    let chunks = d_service
        .get_chunks(document.id, &content, Some(embedder.clone()))
        .await?;

    let create = CreateEmbeddings {
        id: document_id,
        collection: collection.id,
        chunks: chunks.iter().map(|c| c.as_str()).collect(),
    };

    v_service
        .create_embeddings(&*vector_db, &*embedder, create)
        .await?;

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
    state: State<ServiceState>,
    Json(mut search): Json<SearchPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    search.validify()?;

    let service = VectorService::new(state.postgres.clone());

    let search = if let Some(collection_id) = search.collection_id {
        let collection = service.get_collection(collection_id).await?;
        Search {
            query: search.query,
            collection,
            limit: search.limit,
        }
    } else {
        let (Some(name), Some(provider)) = (search.collection_name, search.provider) else {
            // Cannot happen because of above validify
            return Err(ChonkitError::InvalidProvider(
                format!("Both 'collection_name' and 'provider' must be provided if 'collection_id' is not provided"),
            ));
        };

        let collection = service.get_collection_by_name(&name, &provider).await?;

        Search {
            query: search.query,
            collection,
            limit: search.limit,
        }
    };

    let embedder = state.embedder(search.collection.embedder.as_str().try_into()?);
    let vector_db = state.vector_db(search.collection.provider.as_str().try_into()?);

    let chunks = service.search(&*vector_db, &*embedder, search).await?;

    Ok(Json(chunks))
}
