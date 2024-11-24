//! Veloquent 请求处理

pub use super::entity;
pub use crate::{error::AppError, utility};

pub use axum::{
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
pub use axum_extra::protobuf::Protobuf;
pub use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseBackend::Postgres, DatabaseConnection, DeleteResult,
    EntityTrait, FromQueryResult, JoinType, QueryFilter, QuerySelect, QueryTrait, RelationTrait,
    Statement,
};
pub use sea_query::{Alias, Condition};
pub use serde::{Deserialize, Serialize};
pub use std::{collections::HashMap, sync::Arc};
pub use tokio::sync::Mutex;
pub use tracing::{event, instrument, Level};
pub use uuid::Uuid;

use super::jwt::JWTPayload;

#[cfg(feature = "dev")]
use utoipa::OpenApi;
#[cfg(feature = "dev")]
use utoipa::{IntoParams, ToSchema};
#[cfg(feature = "dev")]
use utoipa_swagger_ui::SwaggerUi;

use prost::Message;

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

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct AppState {
    pub conn: DatabaseConnection,
    pub ws_pool: WebSocketPool,
}

#[doc(hidden)]
#[derive(Clone, Debug, Default)]
pub struct WebSocketPool(Arc<Mutex<HashMap<Uuid, WebSocket>>>);

impl WebSocketPool {
    #[instrument(skip(self, ws))]
    pub async fn register(self, user: Uuid, ws: WebSocket) {
        event!(Level::INFO, "register websocket for user [{}]", user);
        self.0.lock().await.insert(user, ws);
    }

    #[instrument(skip(self))]
    pub async fn notify(self, user: Uuid, message: Result<WebSocketMessage, AppError>) {
        if let Some(ws) = self.0.lock().await.get_mut(&user) {
            if let Ok(message) = message {
                event!(
                    Level::INFO,
                    "send message [{:?}] to user [{}]",
                    message,
                    user
                );
                ws.send(message).await.ok();
            }
        }
    }

    pub async fn remove(self, user: Uuid) {
        self.0.lock().await.remove(&user);
    }
}

/// Swagger Open API 文档路径
#[cfg(feature = "dev")]
pub(super) static DOC_PATH: &str = "/doc";

#[instrument(skip(state, ws))]
async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    payload: JWTPayload,
    State(state): State<AppState>,
) -> impl IntoResponse {
    event!(Level::INFO, "receive websocket establishment request");
    ws.on_upgrade(move |socket| state.ws_pool.register(payload.id, socket))
}

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
            post(contact::reject_contact_handler)
                .delete(contact::delete_contact_handler)
                .route_layer(auth.clone()),
        )
        .route(
            "/upload/avatar",
            post(avatar::upload_avatar_handler).route_layer(auth.clone()),
        )
        .route(
            "/download/:id",
            get(download::download_handler).route_layer(auth.clone()),
        )
        .route("/ws", get(ws_upgrade_handler).route_layer(auth.clone()))
        .with_state(state)
}
