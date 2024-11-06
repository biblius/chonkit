use super::{
    api::ApiDoc,
    dto::{CreateCollectionPayload, SearchPayload},
};
use crate::dto::{
    ChunkPreviewPayload, ConfigUpdatePayload, EmbeddingBatchPayload, EmbeddingSinglePayload,
    ListDocumentsPayload, ListEmbeddingsPayload, UploadResult,
};
use axum::{
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderValue, Method},
    response::{sse::Event, IntoResponse, Sse},
    routing::{delete, get, post, put},
    Json, Router,
};
use chonkit::{
    app::{
        batch::{BatchEmbedderHandle, BatchJob, JobResult},
        state::{AppState, DocumentStoreProvider},
    },
    core::{
        document::parser::ParseConfig,
        model::{
            collection::{Collection, CollectionDisplay, Embedding},
            document::{Document, DocumentDisplay, DocumentType},
            List, Pagination,
        },
        service::{
            document::{dto::DocumentUpload, DocumentService},
            vector::{
                dto::{CreateEmbeddings, Search},
                VectorService,
            },
        },
    },
    error::ChonkitError,
};
use futures_util::Stream;
use std::{collections::HashMap, time::Duration};
use tokio_stream::StreamExt;
use tower_http::{classify::ServerErrorsFailureClass, cors::CorsLayer, trace::TraceLayer};
use tracing::{error, Span};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;
use validify::{Validate, Validify};

pub fn router(
    state: AppState,
    batch_embedder: BatchEmbedderHandle,
    origins: Vec<String>,
) -> Router {
    let origins = origins
        .into_iter()
        .map(|origin| {
            tracing::debug!("Adding {origin} to allowed origins");
            HeaderValue::from_str(&origin)
        })
        .map(Result::unwrap);

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::list(origins))
        .allow_headers(tower_http::cors::Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::DELETE,
            Method::PUT,
            Method::PATCH,
        ]);

    service_api(state.clone())
        .merge(batch_api(batch_embedder))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(TraceLayer::new_for_http().on_failure(
            |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                tracing::error!("{error}")
            },
        ))
        .layer(cors)
}

fn service_api(state: AppState) -> Router {
    Router::new()
        .route("/_health", get(health_check))
        .route("/info", get(app_config))
        .route("/documents", get(list_documents))
        .route("/documents", post(upload_documents))
        .layer(DefaultBodyLimit::max(50_000_000))
        .route("/documents/:id", get(get_document))
        .route("/documents/:id", delete(delete_document))
        .route("/documents/:id/config", put(update_document_config))
        .route("/documents/:id/chunk/preview", post(chunk_preview))
        .route("/documents/:id/parse/preview", post(parse_preview))
        .route("/documents/sync/:provider", get(sync))
        .route("/collections", get(list_collections))
        .route("/collections", post(create_collection))
        .route("/collections/:id", get(get_collection))
        .route("/collections/:id", delete(delete_collection))
        .route(
            "/collections/:collection_id/documents/:document_id",
            delete(delete_embeddings),
        )
        .route(
            "/collections/:collection_id/documents/:document_id/count",
            get(count_embeddings),
        )
        .route("/embeddings", get(list_embedded_documents))
        .route("/embeddings", post(embed))
        .route("/embeddings/:provider/models", get(list_embedding_models))
        .route("/search", post(search))
        .route("/display/documents", get(list_documents_display))
        .route("/display/collections", get(list_collections_display))
        .route("/display/collections/:id", get(collection_display))
        .with_state(state)
}

fn batch_api(batch_embedder: BatchEmbedderHandle) -> Router {
    Router::new()
        .route("/embeddings/batch", post(batch_embed))
        .with_state(batch_embedder)
}
// General app configuration

async fn health_check() -> impl IntoResponse {
    "OK"
}

#[utoipa::path(
    get,
    path = "/info",
    responses(
        (status = 200, description = "Get app configuration and available providers", body = AppConfig),
        (status = 500, description = "Internal server error")
    )
)]
async fn app_config(state: State<AppState>) -> Result<impl IntoResponse, ChonkitError> {
    Ok(Json(state.get_configuration().await?))
}

// Document router

#[utoipa::path(
    get,
    path = "/documents",
    responses(
        (status = 200, description = "List documents", body = inline(List<Document>)),
        (status = 400, description = "Invalid pagination parameters"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("pagination" = ListDocumentsPayload, Query, description = "Query parameters"),
    ),
)]
async fn list_documents(
    state: State<AppState>,
    payload: Option<Query<ListDocumentsPayload>>,
) -> Result<Json<List<Document>>, ChonkitError> {
    let Query(pagination) = payload.unwrap_or_default();

    let service = DocumentService::new(state.postgres.clone());

    let documents = service
        .list_documents(pagination.pagination, pagination.src.as_deref())
        .await?;

    Ok(Json(documents))
}

#[utoipa::path(
    get,
    path = "/display/documents",
    responses(
        (status = 200, description = "List documents with additional info for display purposes.", body = inline(List<DocumentDisplay>)),
        (status = 400, description = "Invalid pagination parameters"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("pagination" = ListDocumentsPayload, Query, description = "Query parameters"),
    ),
)]
async fn list_documents_display(
    state: State<AppState>,
    payload: Option<Query<ListDocumentsPayload>>,
) -> Result<Json<List<DocumentDisplay>>, ChonkitError> {
    let Query(payload) = payload.unwrap_or_default();

    let service = DocumentService::new(state.postgres.clone());

    let documents = service
        .list_documents_display(
            payload.pagination,
            payload.src.as_deref(),
            payload.document_id,
        )
        .await?;

    Ok(Json(documents))
}

#[utoipa::path(
    get,
    path = "/display/collections",
    responses(
        (status = 200, description = "List collections with additional info for display purposes.", body = inline(List<CollectionDisplay>)),
        (status = 400, description = "Invalid pagination parameters"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("pagination" = Pagination, Query, description = "Query parameters"),
    ),
)]
async fn list_collections_display(
    state: State<AppState>,
    payload: Option<Query<Pagination>>,
) -> Result<Json<List<CollectionDisplay>>, ChonkitError> {
    let Query(pagination) = payload.unwrap_or_default();

    let service = VectorService::new(state.postgres.clone());

    let collections = service.list_collections_display(pagination).await?;

    Ok(Json(collections))
}

#[utoipa::path(
    get,
    path = "/display/collections/{id}",
    responses(
        (status = 200, description = "Get collection by id", body = CollectionDisplay),
        (status = 404, description = "Collection not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Collection ID")        
    ) 
)]
async fn collection_display(
    state: State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CollectionDisplay>, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());

    let collection = service.get_collection_display(id).await?;

    Ok(Json(collection))
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
    state: axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
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
    state: axum::extract::State<AppState>,
    Path(id): Path<Uuid>,
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
    ),
    request_body = axum::extract::Multipart
)]
async fn upload_documents(
    state: axum::extract::State<AppState>,
    mut form: axum::extract::Multipart,
) -> Result<Json<UploadResult>, ChonkitError> {
    let mut documents = vec![];
    let mut errors = HashMap::<String, Vec<String>>::new();

    let service = DocumentService::new(state.postgres.clone());

    // Only store provider that supports upload currently
    let store = state.store(DocumentStoreProvider::Fs);

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
    path = "/documents/{id}/config",
    responses(
        (status = 200, description = "Update parsing and chunking configuration", body = String),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID"),
    ),
    request_body = ConfigUpdatePayload
)]
async fn update_document_config(
    state: State<AppState>,
    Path(document_id): Path<Uuid>,
    Json(config): Json<ConfigUpdatePayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    let ConfigUpdatePayload { parser, chunker } = config;

    let service = DocumentService::new(state.postgres.clone());

    if let Some(parser) = parser {
        service.update_parser(document_id, parser).await?;
    }

    if let Some(chunker) = chunker {
        service.update_chunker(document_id, chunker).await?;
    }

    Ok(format!(
        "Successfully updated configuration for {document_id}"
    ))
}

#[utoipa::path(
    post,
    path = "/documents/{id}/chunk/preview",
    responses(
        (status = 200, description = "Preview chunk parsing", body = Vec<String>),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID"),
    ),
    request_body = ChunkPreviewPayload
)]
async fn chunk_preview(
    state: State<AppState>,
    Path(id): Path<Uuid>,
    Json(config): Json<ChunkPreviewPayload>,
) -> Result<impl IntoResponse, ChonkitError> {
    config.validate()?;

    let service = DocumentService::new(state.postgres.clone());
    let document = service.get_document(id).await?;

    let store = state.store(document.src.as_str().try_into()?);

    let embedder = if let Some(embedder) = &config.embedder {
        Some(state.embedder(embedder.as_str().try_into()?))
    } else {
        None
    };

    let parser = if let Some(parser) = config.parser {
        parser
    } else {
        let config = service.get_config(id).await?;
        config
            .parse_config
            .ok_or_else(|| ChonkitError::DoesNotExist(format!("Parsing configuration for {id}")))?
    };

    let content = service.parse_preview(&*store, id, parser).await?;
    let chunked = service
        .chunk_preview(&content, config.chunker, embedder)
        .await?;

    match chunked {
        chonkit::core::chunk::ChunkedDocument::Ref(chunked) => {
            Ok(Json(chunked.into_iter().map(String::from).collect()))
        }
        chonkit::core::chunk::ChunkedDocument::Owned(chunked) => Ok(Json(chunked)),
    }
}

#[utoipa::path(
    post,
    path = "/documents/{id}/parse/preview",
    responses(
        (status = 200, description = "Preview document parse result", body = String),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID")
    ),
    request_body(content = ParseConfig, description = "Optional parse configuration for preview")
)]
async fn parse_preview(
    state: State<AppState>,
    Path(id): Path<Uuid>,
    Json(parser): Json<ParseConfig>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = DocumentService::new(state.postgres.clone());
    let document = service.get_document(id).await?;
    let store = state.store(document.src.try_into()?);
    let parsed = service.parse_preview(&*store, id, parser).await?;
    Ok(Json(parsed))
}

#[utoipa::path(
    get,
    path = "/documents/sync/{provider}", 
    responses(
        (status = 200, description = "Successfully synced", body = String),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = String, Path, description = "Storage provider")
    ),
)]
async fn sync(
    state: axum::extract::State<AppState>,
    Path(provider): Path<String>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = DocumentService::new(state.postgres.clone());
    let store = state.store(provider.try_into()?);
    service.sync(&*store).await?;
    Ok("Successfully synced")
}

// Vector router

#[utoipa::path(
    get,
    path = "/collections", 
    responses(
        (status = 200, description = "List collections", body = inline(List<Collection>)),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("pagination" = Pagination, Query, description = "Pagination parameters")
    )
)]
async fn list_collections(
    state: State<AppState>,
    Query(p): Query<Pagination>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let collections = service.list_collections(p).await?;
    Ok(Json(collections))
}

#[utoipa::path(
    post,
    path = "/collections", 
    responses(
        (status = 200, description = "Collection created successfully", body = Collection),
        (status = 500, description = "Internal server error")
    ),
    request_body = CreateCollectionPayload
)]
async fn create_collection(
    state: State<AppState>,
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
    path = "/collections/{id}", 
    responses(
        (status = 200, description = "Collection retrieved successfully", body = Collection),
        (status = 404, description = "Collection not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Collection ID")
    )
)]
async fn get_collection(
    state: State<AppState>,
    Path(collection_id): Path<Uuid>,
) -> Result<Json<Collection>, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let collection = service.get_collection(collection_id).await?;
    Ok(Json(collection))
}

#[utoipa::path(
    delete,
    path = "/collections/{id}", 
    responses(
        (status = 200, description = "Collection deleted successfully", body = String),
        (status = 404, description = "Collection not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Collection ID")
    )
)]
async fn delete_collection(
    state: State<AppState>,
    Path(collection_id): Path<Uuid>,
) -> Result<String, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let collection = service.get_collection(collection_id).await?;
    let vector_db = state.vector_db(collection.provider.try_into()?);
    service
        .delete_collection(&*vector_db, collection_id)
        .await?;
    Ok(format!(
        "Successfully deleted collection with ID '{collection_id}'"
    ))
}

#[utoipa::path(
    get,
    path = "/embeddings/{provider}/models", 
    responses(
        (status = 200, description = "List available embedding models", body = HashMap<String, usize>),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("provider" = String, Path, description = "Vector database provider"),
    ),
)]
async fn list_embedding_models(
    state: State<AppState>,
    Path(provider): Path<String>,
) -> Result<Json<HashMap<String, usize>>, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let embedder = state.embedder(provider.as_str().try_into()?);
    let models = service
        .list_embedding_models(&*embedder)
        .await?
        .into_iter()
        .collect::<HashMap<String, usize>>();
    Ok(Json(models))
}

#[utoipa::path(
    post,
    path = "/embeddings", 
    responses(
        (status = 200, description = "Embeddings created successfully", body = String),
        (status = 404, description = "Collection or document not found"),
        (status = 500, description = "Internal server error")
    ),
    request_body = EmbeddingSinglePayload
)]
async fn embed(
    state: axum::extract::State<AppState>,
    Json(payload): Json<EmbeddingSinglePayload>,
) -> Result<&'static str, ChonkitError> {
    let EmbeddingSinglePayload {
        document: document_id,
        collection,
    } = payload;

    let d_service = DocumentService::new(state.postgres.clone());
    let v_service = VectorService::new(state.postgres.clone());

    let document = d_service.get_document(document_id).await?;
    let collection = v_service.get_collection(collection).await?;

    let store = state.store(document.src.as_str().try_into()?);
    let vector_db = state.vector_db(collection.provider.as_str().try_into()?);
    let embedder = state.embedder(collection.embedder.as_str().try_into()?);

    let content = d_service.get_content(&*store, document_id).await?;
    let chunks = d_service
        .get_chunks(document.id, &content, Some(embedder.clone()))
        .await?;

    let chunks = match chunks {
        chonkit::core::chunk::ChunkedDocument::Ref(r) => r,
        chonkit::core::chunk::ChunkedDocument::Owned(ref o) => {
            o.iter().map(|s| s.as_str()).collect()
        }
    };

    let create = CreateEmbeddings {
        id: document.id,
        collection: collection.id,
        chunks: &chunks,
    };

    v_service
        .create_embeddings(&*vector_db, &*embedder, create)
        .await?;

    Ok("Successfully created embeddings")
}

#[utoipa::path(
    post,
    path = "/embeddings/batch", 
    responses(
        (status = 200, description = "Embeddings created successfully"),
        (status = 500, description = "Internal server error")
    ),
    request_body = EmbeddingBatchPayload
)]
async fn batch_embed(
    State(batch_embedder): axum::extract::State<BatchEmbedderHandle>,
    Json(job): Json<EmbeddingBatchPayload>,
) -> Result<Sse<impl Stream<Item = Result<Event, ChonkitError>>>, ChonkitError> {
    job.validate()?;

    let EmbeddingBatchPayload {
        collection,
        add,
        remove
    } = job;

    let (tx, rx) = tokio::sync::mpsc::channel::<JobResult>(add.len() + remove.len());

    let job = BatchJob::new(collection, add, remove, tx);

    if let Err(e) = batch_embedder.send(job).await {
        error!("Error sending embedding job: {:?}", e.0);
        return Err(ChonkitError::Batch);
    };

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(|result| {
        let event = match result {
            JobResult::Ok(report) => Event::default().json_data(report)?,
            JobResult::Err(err) => {
                tracing::error!("Received error in batch embedder: {err}");
                let err = format!("error: {err}");
                Event::default().data(err)
            }
        };
        Ok(event)
    });

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive"),
    ))
}

#[utoipa::path(
    get,
    path = "/embeddings", 
    responses(
        (status = 200, description = "List of embedded documents, optionally filtered by collection ID", body = inline(List<Embedding>)),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("payload" = ListEmbeddingsPayload, Query, description = "List parameters"),
    ),
)]
async fn list_embedded_documents(
    state: axum::extract::State<AppState>,
    Query(payload): Query<ListEmbeddingsPayload>,
) -> Result<Json<List<Embedding>>, ChonkitError> {
    payload.validate()?;

    let ListEmbeddingsPayload {
        collection: collection_id,
        pagination,
    } = payload;

    let service = VectorService::new(state.postgres.clone());
    let embeddings = service.list_embeddings(pagination, collection_id).await?;

    Ok(Json(embeddings))
}

#[utoipa::path(
    post,
    path = "/search", 
    responses(
        (status = 200, description = "Search results returned"),
        (status = 500, description = "Internal server error")
    ),
    request_body = SearchPayload
)]
async fn search(
    state: State<AppState>,
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

#[utoipa::path(
    get,
    path = "/collections/{collection_id}/documents/{document_id}/count",
    responses(
        (status = 200, description = "Count of embeddings for a given document in a given collection.", body = Json<usize>),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("collection_id" = Uuid, Path, description = "Collection ID"),
        ("document_id" = Uuid, Path, description = "Document ID"),
    ),
)]
async fn count_embeddings(
    state: State<AppState>,
    Path((collection_id, document_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let embeddings = service
        .get_embeddings(document_id, collection_id)
        .await?
        .ok_or_else(|| {
            ChonkitError::DoesNotExist(format!(
                "Embeddings for document '{document_id}' in collection '{collection_id}'"
            ))
        })?;
    let collection = service.get_collection(embeddings.collection_id).await?;
    let vector_db = state.vector_db(collection.provider.as_str().try_into()?);
    let amount = service
        .count_embeddings(collection_id, document_id, &*vector_db)
        .await?;

    Ok(Json(amount))
}

#[utoipa::path(
    delete,
    path = "/collections/{collection_id}/documents/{document_id}",
    responses(
        (status = 200, description = "Delete embeddings for a given document in a given collection.", body = String),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("collection_id" = Uuid, Path, description = "Collection ID"),
        ("document_id" = Uuid, Path, description = "Document ID"),
    ),
)]
async fn delete_embeddings(
    state: State<AppState>,
    Path((collection_id, document_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ChonkitError> {
    let service = VectorService::new(state.postgres.clone());
    let embeddings = service
        .get_embeddings(document_id, collection_id)
        .await?
        .ok_or_else(|| {
            ChonkitError::DoesNotExist(format!(
                "Embeddings for document '{document_id}' in collection '{collection_id}'"
            ))
        })?;
    let collection = service.get_collection(embeddings.collection_id).await?;
    let vector_db = state.vector_db(collection.provider.as_str().try_into()?);
    let amount = service
        .delete_embeddings(collection_id, document_id, &*vector_db)
        .await?;
    Ok(format!("Successfully deleted {amount} embedding(s)"))
}
