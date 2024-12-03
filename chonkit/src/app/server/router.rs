use super::api::ApiDoc;
use crate::{app::state::AppState, error::ChonkitError};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderValue, Method},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use std::time::Duration;
use tower_http::{classify::ServerErrorsFailureClass, cors::CorsLayer, trace::TraceLayer};
use tracing::Span;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub(super) mod document;
pub(super) mod vector;

pub fn router(state: AppState, origins: Vec<String>) -> Router {
    let origins = origins
        .into_iter()
        .map(|origin| {
            tracing::info!("Adding {origin} to allowed origins");
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

    use document::*;
    use vector::*;

    let sync = Router::new()
        .route("/documents/sync/:provider", get(document::sync))
        .route("/info", get(app_config))
        .with_state(state.clone());

    let batch_router = Router::new()
        .route("/embeddings/batch", post(batch_embed))
        .with_state(state.batch_embedder.clone());

    let router = Router::new()
        .route("/documents", get(list_documents))
        .route("/documents", post(upload_documents))
        .layer(DefaultBodyLimit::max(50_000_000))
        .route("/documents/:id", get(get_document))
        .route("/documents/:id", delete(delete_document))
        .route("/documents/:id/config", put(update_document_config))
        .route("/documents/:id/chunk/preview", post(chunk_preview))
        .route("/documents/:id/parse/preview", post(parse_preview))
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
        .with_state(state.services.clone())
        .merge(batch_router)
        .merge(sync);

    #[cfg(feature = "auth-vault")]
    let router = router.layer(axum::middleware::from_fn_with_state(
        state.vault.clone(),
        crate::app::auth::auth_check,
    ));

    router
        .layer(
            TraceLayer::new_for_http()
                .on_request(|req: &axum::http::Request<_>, _span: &Span| {
                    let ctype = req
                        .headers()
                        .get("content-type")
                        .map(|v| v.to_str().unwrap_or_else(|_| "none"))
                        .unwrap_or_else(|| "none");

                    tracing::info!("Processing request | content-type: {ctype}");
                })
                .on_response(
                    |res: &axum::http::Response<_>, latency: Duration, _span: &Span| {
                        let status = res.status();
                        let ctype = res
                            .headers()
                            .get("content-type")
                            .map(|v| v.to_str().unwrap_or_else(|_| "none"))
                            .unwrap_or_else(|| "none");

                        tracing::info!(
                            "Sending response | {status} | {}ms | {ctype}",
                            latency.as_millis()
                        );
                    },
                )
                .on_failure(
                    |error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        tracing::error!("Error in request: {error}")
                    },
                ),
        )
        .layer(cors)
        // Unprotected at all times
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Has to go last to exclude all the tracing/cors layers
        .route("/_health", get(health_check))
}

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
