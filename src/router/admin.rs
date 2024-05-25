use crate::{
    auth::{Auth, AuthError},
    error::LedgeknawError,
    llm::chunk::{ChunkConfig, Chunker, Recursive, SlidingWindow, SnappingWindow},
    state::DocumentService,
};
use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, get_service, post},
    Json, Router,
};
use axum_extra::{headers::Cookie, TypedHeader};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::services::{ServeDir, ServeFile};

pub(super) fn admin_router(documents: DocumentService, auth: Auth) -> Router {
    let auth = Arc::new(auth);

    let router_static = Router::new()
        .nest_service("/", ServeDir::new("public/admin"))
        .with_state(documents.clone())
        .layer(middleware::from_fn_with_state(
            auth.clone(),
            auth::session_check,
        ));

    let router_admin = Router::new().layer(middleware::from_fn_with_state(
        auth.clone(),
        auth::session_check,
    ));

    let router_auth = auth::admin_auth_router(auth);

    let router = router_admin
        .merge(router_static)
        .with_state(documents)
        .merge(router_auth);

    Router::new().nest("/admin", router)
}

mod auth {
    use super::*;

    pub(super) fn admin_auth_router(auth: Arc<Auth>) -> Router {
        Router::new()
            .route(
                "/login",
                get_service(ServeFile::new("public/admin/login.html")),
            )
            .route("/login", post(login))
            .with_state(auth)
    }

    async fn login(
        auth: axum::extract::State<Arc<Auth>>,
        password: axum::extract::Json<String>,
    ) -> Result<Response, LedgeknawError> {
        let result = auth.verify_password(&password);

        if !result {
            return Ok(StatusCode::UNAUTHORIZED.into_response());
        }

        let session = auth.create_session().await?;
        let cookie = auth.create_session_cookie(session.id);

        Ok((StatusCode::OK, [(header::SET_COOKIE, cookie.to_string())]).into_response())
    }

    pub(super) async fn session_check(
        auth: axum::extract::State<Arc<Auth>>,
        cookie: TypedHeader<Cookie>,
        req: Request,
        next: Next,
    ) -> Result<impl IntoResponse, LedgeknawError> {
        let cookie = cookie.0.get("SID");

        let Some(cookie) = cookie else {
            return Err(AuthError::NoSession.into());
        };

        let Ok(session_id) = uuid::Uuid::parse_str(cookie) else {
            return Err(AuthError::NoSession.into());
        };

        let valid_exists = auth.session_check(session_id).await?;

        if !valid_exists {
            return Err(AuthError::NoSession.into());
        }

        let response = next.run(req).await;

        Ok(response)
    }
}
