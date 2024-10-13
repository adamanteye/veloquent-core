//! Veloquent 请求处理

use axum::{routing::post, Router};
use sea_orm::DatabaseConnection;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub(super) mod login;
pub(super) mod user;

use super::openapi::ApiDoc;

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct AppState {
    pub conn: DatabaseConnection,
}

/// Veloquent 路由
pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/doc").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/api/login", post(login::login_handler))
        .with_state(state)
}
