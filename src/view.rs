//! Veloquent 请求处理

use super::entity;
use super::jwt::JWTPayload;
use crate::{error::AppError, utility};
use ws::WebSocketPool;

use axum::{
    body::Bytes,
    extract::{
        ws::{Message as WebSocketMessage, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use axum_extra::protobuf::Protobuf;
use dashmap::DashMap;
use futures::stream::SplitSink;
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseBackend::Postgres, DatabaseConnection, DeleteResult,
    EntityTrait, FromQueryResult, IntoActiveModel, JoinType, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Statement,
};
use sea_query::Condition;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[cfg(feature = "dev")]
use utoipa::OpenApi;
#[cfg(feature = "dev")]
use utoipa::{IntoParams, ToSchema};
#[cfg(feature = "dev")]
use utoipa_swagger_ui::SwaggerUi;

mod avatar;
mod contact;
mod download;
mod feed;
mod group;
mod history;
mod login;
mod message;
#[cfg(feature = "dev")]
mod openapi;
mod user;
mod ws;

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct AppState {
    pub conn: DatabaseConnection,
    pub ws_pool: WebSocketPool,
}

/// Swagger Open API 文档路径
#[cfg(feature = "dev")]
pub static DOC_PATH: &str = "/doc";

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
        .route("/renew", get(login::renew_handler))
        .route("/register", post(user::register_handler))
        .route(
            "/user",
            get(user::find_user_handler).route_layer(auth.clone()),
        )
        .route(
            "/user/profile",
            get(user::get_profile_handler)
                .delete(user::delete_user_handler)
                .put(user::update_profile_handler)
                .route_layer(auth.clone()),
        )
        .route(
            "/contact/list",
            get(contact::get_contacts_handler).route_layer(auth.clone()),
        )
        .route(
            "/contact/categories",
            get(contact::get_categories_handler).route_layer(auth.clone()),
        )
        .route(
            "/contact/pending",
            get(contact::get_pending_contacts_handler).route_layer(auth.clone()),
        )
        .route(
            "/contact/new",
            get(contact::get_new_contacts_handler).route_layer(auth.clone()),
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
            "/contact/reject/:id",
            put(contact::reject_contact_handler).route_layer(auth.clone()),
        )
        .route(
            "/contact/delete/:id",
            delete(contact::delete_contact_handler).route_layer(auth.clone()),
        )
        .route(
            "/contact/edit/:id",
            put(contact::edit_contact_handler).route_layer(auth.clone()),
        )
        .route(
            "/msg/:id",
            get(message::get_msg_handler)
                .delete(message::delete_msg_handler)
                .route_layer(auth.clone()),
        )
        .route(
            "/msg/session/:id",
            post(message::send_msg_handler)
                .get(history::get_history_handler)
                .route_layer(auth.clone()),
        )
        .route(
            "/msg/mask/:id",
            put(message::mask_msg_handler).route_layer(auth.clone()),
        )
        .route(
            "/group/:id",
            get(group::get_group_handler)
                .delete(group::delete_group_handler)
                .route_layer(auth.clone()),
        )
        .route(
            "/group/new",
            post(group::create_group_handler).route_layer(auth.clone()),
        )
        .route(
            "/group/list",
            get(group::list_group_handler).route_layer(auth.clone()),
        )
        .route(
            "/group/manage/:id",
            get(group::monitor_group_handler)
                .put(group::manage_group_handler)
                .route_layer(auth.clone()),
        )
        .route(
            "/group/exit/:id",
            delete(group::exit_group_handler).route_layer(auth.clone()),
        )
        .route(
            "/group/edit/:id",
            put(group::pin_group_handler).route_layer(auth.clone()),
        )
        .route(
            "/group/invite/:id",
            post(group::invite_group_handler).route_layer(auth.clone()),
        )
        .route(
            "/group/approve/:id",
            put(group::approve_group_handler).route_layer(auth.clone()),
        )
        .route(
            "/upload",
            post(avatar::upload_handler).route_layer(auth.clone()),
        )
        .route(
            "/upload/avatar",
            post(avatar::upload_avatar_handler).route_layer(auth.clone()),
        )
        .route(
            "/download/:id",
            get(download::download_handler).route_layer(auth.clone()),
        )
        .route("/ws", get(ws::ws_upgrade_handler))
        .with_state(state)
}
