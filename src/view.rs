//! Veloquent 请求处理

pub use super::entity;
pub use crate::{error::AppError, utility};

pub use axum::{
    extract::State,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
pub use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
pub use serde::{Deserialize, Serialize};
pub use tracing::{event, instrument, Level};
pub use utoipa::ToSchema;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod login;
mod openapi;
mod user_profile;
mod user_register;

use super::jwt::JWTPayload;

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct AppState {
    pub conn: DatabaseConnection,
}

/// Swagger Open API 文档路径
pub(super) static DOC_PATH: &str = "/doc";

/// Veloquent 路由
pub fn router(state: AppState) -> Router {
    let auth = middleware::from_extractor::<JWTPayload>();
    Router::new()
        .merge(SwaggerUi::new(DOC_PATH).url("/api-docs/openapi.json", openapi::ApiDoc::openapi()))
        .route("/login", post(login::login_handler))
        .route("/register", post(user_register::register_handler))
        .route(
            "/user/profile",
            get(user_profile::get_self_profile).route_layer(auth),
        )
        .with_state(state)
}
