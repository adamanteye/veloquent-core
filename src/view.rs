//! Veloquent 请求处理

pub use axum::{
    extract::State,
    middleware,
    routing::{get, post},
    Router,
};
use sea_orm::DatabaseConnection;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub(super) mod login;
pub(super) mod user_profile;

use super::{jwt::JWTPayload, openapi::ApiDoc};

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct AppState {
    pub conn: DatabaseConnection,
}

/// Veloquent 路由
pub fn router(state: AppState) -> Router {
    let auth = middleware::from_extractor::<JWTPayload>();
    Router::new()
        .merge(SwaggerUi::new("/doc").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/login", post(login::login_handler))
        .route(
            "/user/profile",
            get(user_profile::get_self_profile).route_layer(auth),
        )
        .with_state(state)
}
