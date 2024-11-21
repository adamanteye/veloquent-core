//! Veloquent 请求处理

pub use super::entity;
pub use crate::{error::AppError, utility};

pub use axum::{
    body::Bytes,
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
pub use axum_extra::protobuf::Protobuf;
pub use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseBackend::Postgres, DatabaseConnection, DeleteResult,
    EntityTrait, FromQueryResult, JoinType, QueryFilter, QuerySelect, QueryTrait, RelationTrait,
    Statement,
};
pub use sea_query::{Alias, Condition};
pub use serde::{Deserialize, Serialize};
pub use tracing::{event, instrument, Level};
#[cfg(feature = "dev")]
use utoipa::OpenApi;
#[cfg(feature = "dev")]
use utoipa::{IntoParams, ToSchema};
#[cfg(feature = "dev")]
use utoipa_swagger_ui::SwaggerUi;
pub use uuid::Uuid;

mod avatar;
mod contact;
mod download;
mod login;
#[cfg(feature = "dev")]
mod openapi;
mod user_delete;
mod user_find;
mod user_profile;
mod user_register;

use super::jwt::JWTPayload;

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct AppState {
    pub conn: DatabaseConnection,
}

/// Swagger Open API 文档路径
#[cfg(feature = "dev")]
pub(super) static DOC_PATH: &str = "/doc";

/// Veloquent 路由
pub fn router(state: AppState) -> Router {
    let auth = middleware::from_extractor::<JWTPayload>();
    let router = {
        #[cfg(feature = "dev")]
        {
            Router::new().merge(
                SwaggerUi::new(DOC_PATH).url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
            )
        }
        #[cfg(not(feature = "dev"))]
        {
            Router::new()
        }
    };

    router
        .route("/login", post(login::login_handler))
        .route("/register", post(user_register::register_handler))
        .route(
            "/user",
            get(user_find::find_user_handler).route_layer(auth.clone()),
        )
        .route(
            "/user/profile",
            get(user_profile::get_profile_handler)
                .delete(user_delete::delete_user_handler)
                .put(user_profile::update_profile_handler)
                .route_layer(auth.clone()),
        )
        .route(
            "/contact/list",
            get(contact::get_contacts_handler).route_layer(auth.clone()),
        )
        .route(
            "/contact/pending",
            get(contact::get_pending_contacts_handler).route_layer(auth.clone()),
        )
        .route(
            "/contact/new/:id",
            post(contact::add_contact_handler).route_layer(auth.clone()),
        )
        .route(
            "/contact/accept/:id",
            post(contact::accept_contact_handler).route_layer(auth.clone()),
        )
        .route(
            "/contact/delete/:id",
            delete(contact::delete_contact_handler).route_layer(auth.clone()),
        )
        .route(
            "/upload/avatar",
            post(avatar::upload_avatar_handler).route_layer(auth.clone()),
        )
        .route(
            "/download/:id",
            get(download::download_handler).route_layer(auth.clone()),
        )
        .with_state(state)
}
