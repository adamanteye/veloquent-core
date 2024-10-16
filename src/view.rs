//! Veloquent 请求处理

pub use super::entity;
pub use crate::{error::AppError, utility};

pub use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
pub use axum_extra::protobuf::Protobuf;
pub use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, DeleteResult, EntityTrait, QueryFilter,
};
pub use serde::{Deserialize, Serialize};
pub use tracing::{event, instrument, Level};
pub use utoipa::{IntoParams, ToSchema};
pub use uuid::Uuid;

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod avatar;
mod login;
mod openapi;
mod user_delete;
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
            get(user_profile::get_self_profile_handler)
                .delete(user_delete::delete_user_handler)
                .route_layer(auth.clone()),
        )
        .route(
            "/upload/avatar",
            post(avatar::upload_avatar_handler).route_layer(auth),
        )
        .with_state(state)
}
