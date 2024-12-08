//! Veloquent 请求处理

pub use super::entity;
pub use crate::{error::AppError, utility};

pub use axum::{
    body::Bytes,
    extract::{
        ws::{CloseFrame, Message as WebSocketMessage, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{any, delete, get, post, put},
    Json, Router,
};
pub use axum_extra::protobuf::Protobuf;
pub use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseBackend::Postgres, DatabaseConnection, DeleteResult,
    EntityTrait, FromQueryResult, IntoActiveModel, JoinType, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, RelationTrait, Statement,
};
pub use sea_query::{Alias, Condition};
pub use serde::{Deserialize, Serialize};
pub use std::{collections::HashMap, sync::Arc};
pub use tokio::sync::{Mutex, RwLock};
pub use tracing::{event, instrument, Level};
pub use uuid::Uuid;

use super::jwt::{JWTPayload, JWT_ALG, JWT_SETTING};

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

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct AppState {
    pub conn: DatabaseConnection,
    pub ws_pool: WebSocketPool,
}

#[doc(hidden)]
#[derive(Clone, Debug, Default)]
pub struct WebSocketPool(Arc<Mutex<HashMap<Uuid, Arc<Mutex<WebSocket>>>>>);

impl WebSocketPool {
    #[instrument(skip(self, ws))]
    pub async fn register(self, user: Uuid, ws: WebSocket) {
        event!(Level::INFO, "register websocket for user [{}]", user);
        let mut map = self.0.lock().await;
        map.insert(user, Arc::new(Mutex::new(ws)));
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
                ws.lock().await.send(message).await.ok();
            }
        }
    }
}

async fn handle_socket(mut socket: WebSocket, pool: WebSocketPool) {
    if let Some(msg) = socket.recv().await {
        event!(Level::DEBUG, "received message: [{:?}]", msg);
        if let Ok(msg) = msg {
            match msg {
                WebSocketMessage::Text(t) => {
                    let token = jsonwebtoken::decode::<JWTPayload>(
                        &t,
                        &JWT_SETTING.get().unwrap().de_key,
                        JWT_ALG.get().unwrap(),
                    )
                    .map_err(|e| AppError::Unauthorized(format!("invalid JWT: [{}]", e)));
                    match token {
                        Ok(token) => {
                            let payload = token.claims;
                            event!(Level::INFO, "received user: [{}]", payload.id);
                            pool.register(payload.id, socket).await;
                        }
                        Err(e) => {
                            event!(Level::ERROR, "invalid JWT: [{:?}]", e);
                            return;
                        }
                    }
                }
                _ => {
                    return;
                }
            }
        } else {
            return;
        }
    }
}

/// Swagger Open API 文档路径
#[cfg(feature = "dev")]
pub(super) static DOC_PATH: &str = "/doc";

#[instrument(skip(state, ws))]
async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    event!(Level::INFO, "receive websocket establishment request");
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state.ws_pool)))
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
            "/group/transfer",
            put(group::transfer_group_handler).route_layer(auth.clone()),
        )
        .route(
            "/group/exit/:id",
            delete(group::exit_group_handler).route_layer(auth.clone()),
        )
        .route(
            "/group/pin/:id",
            put(group::pin_group_handler).route_layer(auth.clone()),
        )
        .route(
            "/group/invite/:id",
            post(group::invite_group_handler).route_layer(auth.clone()),
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
        .route("/ws", get(ws_upgrade_handler))
        .with_state(state)
}
