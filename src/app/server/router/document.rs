use crate::{
    app::{
        server::dto::{ConfigUpdatePayload, ListDocumentsPayload, UploadResult},
        state::{GlobalState, ServiceState},
    },
    core::{
        document::parser::ParseConfig,
        model::{
            document::{Document, DocumentConfig, DocumentDisplay, DocumentType},
            List,
        },
        service::document::dto::{ChunkPreviewPayload, DocumentUpload},
    },
    error::ChonkitError,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::collections::HashMap;
use uuid::Uuid;

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
pub(super) async fn list_documents(
    services: State<ServiceState>,
    params: Option<Query<ListDocumentsPayload>>,
) -> Result<Json<List<Document>>, ChonkitError> {
    let Query(params) = params.unwrap_or_default();

    let documents = services
        .document
        .list_documents(params.pagination, params.src.as_deref(), params.ready)
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
pub(super) async fn list_documents_display(
    services: State<ServiceState>,
    payload: Option<Query<ListDocumentsPayload>>,
) -> Result<Json<List<DocumentDisplay>>, ChonkitError> {
    let Query(payload) = payload.unwrap_or_default();

    let documents = services
        .document
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
pub(super) async fn get_document(
    services: axum::extract::State<ServiceState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DocumentConfig>, ChonkitError> {
    let document = services.document.get_config(id).await?;
    Ok(Json(document))
}

#[utoipa::path(
    delete,
    path = "/documents/{id}",
    responses(
        (status = 204, description = "Delete document by id"),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID")
    )
)]
pub(super) async fn delete_document(
    services: axum::extract::State<ServiceState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ChonkitError> {
    services.document.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
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
pub(super) async fn upload_documents(
    services: axum::extract::State<ServiceState>,
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
                tracing::error!("error in form: {e}");
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
                tracing::error!("{e}");
                errors
                    .entry(name)
                    .and_modify(|entry| entry.push(e.to_string()))
                    .or_insert_with(|| vec![e.to_string()]);
                continue;
            }
        };

        let upload = DocumentUpload::new(name.to_string(), typ, &file);

        // Only store provider that supports upload currently
        let document = services.document.upload("fs", upload).await?;

        documents.push(document);
    }

    Ok(Json(UploadResult { documents, errors }))
}

#[utoipa::path(
    put,
    path = "/documents/{id}/config",
    responses(
        (status = 204, description = "Update parsing and chunking configuration", body = String),
        (status = 404, description = "Document not found"),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = Uuid, Path, description = "Document ID"),
    ),
    request_body = ConfigUpdatePayload
)]
pub(super) async fn update_document_config(
    services: State<ServiceState>,
    Path(document_id): Path<Uuid>,
    Json(config): Json<ConfigUpdatePayload>,
) -> Result<StatusCode, ChonkitError> {
    let ConfigUpdatePayload { parser, chunker } = config;

    if let Some(parser) = parser {
        services.document.update_parser(document_id, parser).await?;
    }

    if let Some(chunker) = chunker {
        services
            .document
            .update_chunker(document_id, chunker)
            .await?;
    }

    Ok(StatusCode::NO_CONTENT)
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
pub(super) async fn chunk_preview(
    services: State<ServiceState>,
    Path(id): Path<Uuid>,
    Json(config): Json<ChunkPreviewPayload>,
) -> Result<Json<Vec<String>>, ChonkitError> {
    let chunks = services.document.chunk_preview(id, config).await?;
    Ok(Json(chunks))
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
pub(super) async fn parse_preview(
    services: State<ServiceState>,
    Path(id): Path<Uuid>,
    Json(parser): Json<ParseConfig>,
) -> Result<Json<String>, ChonkitError> {
    let parsed = services.document.parse_preview(id, parser).await?;
    Ok(Json(parsed))
}

#[utoipa::path(
    get,
    path = "/documents/sync/{provider}", 
    responses(
        (status = 204, description = "Successfully synced", body = String),
        (status = 500, description = "Internal server error")
    ),
    params(
        ("id" = String, Path, description = "Storage provider")
    ),
)]
pub(super) async fn sync(
    state: axum::extract::State<GlobalState>,
    Path(provider): Path<String>,
) -> Result<StatusCode, ChonkitError> {
    let syncer = state.app_state.syncer(&provider)?;
    state.service_state.document.sync(&*syncer).await?;
    Ok(StatusCode::NO_CONTENT)
}
