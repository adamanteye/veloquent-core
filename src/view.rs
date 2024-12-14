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

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    use super::user;
    use crate::{jwt, utility};
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use futures::{SinkExt, StreamExt};
    use http_body_util::BodyExt;
    use hyper::body::Buf;
    use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseBackend};
    use tokio_tungstenite::tungstenite;

    async fn connect_db_from_env() -> DatabaseConnection {
        use std::env;
        let mut opt = ConnectOptions::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            env::var("DB_USER").unwrap(),
            env::var("DB_PASS").unwrap(),
            env::var("DB_HOST").unwrap(),
            env::var("DB_PORT").unwrap(),
            env::var("DB_NAME").unwrap(),
        ));
        opt.max_connections(10);
        Database::connect(opt).await.unwrap()
    }

    async fn create_app_state() -> AppState {
        use migration::MigratorTrait;
        let conn = connect_db_from_env().await;
        migration::Migrator::down(&conn, Some(8)).await.unwrap();
        conn.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";".to_owned(),
        ))
        .await
        .unwrap();
        migration::Migrator::up(&conn, None).await.unwrap();
        AppState {
            conn: connect_db_from_env().await,
            ws_pool: WebSocketPool::default(),
        }
    }

    fn init_constants() {
        let secret = "secret";
        jwt::JWT_SETTING.get_or_init(|| jwt::JwtSetting {
            exp: 3600,
            de_key: jwt::DecodingKey::from_secret(secret.as_bytes()),
            en_key: jwt::EncodingKey::from_secret(secret.as_bytes()),
        });
        utility::UPLOAD_DIR.get_or_init(|| "upload".to_string());
    }

    async fn start_http_server(addr: &str) -> anyhow::Result<()> {
        init_constants();
        let state = create_app_state().await;
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let app = router(state);
        axum::serve(listener, app)
            .with_graceful_shutdown(utility::shutdown_signal())
            .await?;
        Ok(())
    }

    #[cfg(feature = "dev")]
    fn request_doc(addr: &str) -> Request<Body> {
        Request::builder()
            .uri(format!("{addr}/doc/"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_post_json() -> http::request::Builder {
        Request::builder()
            .header("Content-Type", "application/json")
            .method("POST")
    }

    fn request_register(addr: &str, user: user::RegisterProfile) -> Request<Body> {
        request_post_json()
            .uri(format!("{addr}/register"))
            .body(Body::from(serde_json::to_vec(&user).unwrap()))
            .unwrap()
    }

    fn request_login(addr: &str) -> Request<Body> {
        request_post_json()
            .uri(format!("{addr}/login"))
            .body(Body::from(
                serde_json::to_vec(&login::LoginRequest {
                    name: "test_user_1".to_string(),
                    password: "123456".to_string(),
                })
                .unwrap(),
            ))
            .unwrap()
    }

    fn request_add_contact(addr: &str, token: &str, id: Uuid) -> Request<Body> {
        request_post_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/contact/new/{id}"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_get_json() -> http::request::Builder {
        Request::builder()
            .header("Content-Type", "application/json")
            .method("GET")
    }

    fn request_get_new_contacts(addr: &str, token: &str) -> Request<Body> {
        request_get_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/contact/new"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_put_json() -> http::request::Builder {
        Request::builder()
            .header("Content-Type", "application/json")
            .method("PUT")
    }

    fn request_reject_contact(addr: &str, token: &str, id: Uuid) -> Request<Body> {
        request_put_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/contact/reject/{id}"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_accept_contact(addr: &str, token: &str, id: Uuid) -> Request<Body> {
        request_post_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/contact/accept/{id}"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_get_contacts(addr: &str, token: &str) -> Request<Body> {
        request_get_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/contact/list"))
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn integration() {
        let addr = "127.0.0.1:8000";
        tokio::spawn(start_http_server(addr));
        let addr = format!("http://{addr}");
        tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
        let client =
            hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
                .build_http();
        // test if swagger doc is available
        #[cfg(feature = "dev")]
        {
            let response = client.request(request_doc(&addr)).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
        // test if register is available
        let response = client
            .request(request_register(
                &addr,
                user::RegisterProfile {
                    name: "test_user_1".to_string(),
                    alias: None,
                    phone: "18999990000".to_string(),
                    gender: Some(1),
                    bio: None,
                    link: None,
                    password: "123456".to_string(),
                    email: "test@example.com".to_string(),
                },
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        // test if register with same email is rejected
        let response = client
            .request(request_register(
                &addr,
                user::RegisterProfile {
                    name: "test_user_2".to_string(),
                    alias: None,
                    phone: "18999990001".to_string(),
                    gender: Some(1),
                    bio: None,
                    link: None,
                    password: "123456".to_string(),
                    email: "test@example.com".to_string(),
                },
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        // test if register with bad phone is rejected
        let response = client
            .request(request_register(
                &addr,
                user::RegisterProfile {
                    name: "test_user_3".to_string(),
                    alias: None,
                    phone: "18999990".to_string(),
                    gender: Some(1),
                    bio: None,
                    link: None,
                    password: "123456".to_string(),
                    email: "test_3@example.com".to_string(),
                },
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        // test if login is available
        let response = client.request(request_login(&addr)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let token = response.into_body().collect().await.unwrap().aggregate();
        let token: login::LoginResponse = serde_json::from_reader(token.reader()).unwrap();
        let user_1_token = token.token;
        let user_1 = jwt::JWTPayload::try_from(user_1_token.as_str()).unwrap().id;
        let (mut socket_1, _response) =
            tokio_tungstenite::connect_async(format!("ws://127.0.0.1:8000/ws"))
                .await
                .unwrap();
        assert!(socket_1
            .send(tungstenite::Message::text(&user_1_token))
            .await
            .is_ok());
        let res_to_json = |res: Response<hyper::body::Incoming>| async {
            res.into_body()
                .collect()
                .await
                .unwrap()
                .aggregate()
                .reader()
        };
        let user_2: login::LoginResponse = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_register(
                        &addr,
                        user::RegisterProfile {
                            name: "test_user_2".to_string(),
                            alias: None,
                            phone: "18999990002".to_string(),
                            gender: Some(1),
                            bio: None,
                            link: None,
                            password: "123456".to_string(),
                            email: "test_2@example.com".to_string(),
                        },
                    ))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        let user_2_token = user_2.token;
        let user_2 = jwt::JWTPayload::try_from(user_2_token.as_str()).unwrap().id;
        let (mut socket_2, _response) =
            tokio_tungstenite::connect_async(format!("ws://127.0.0.1:8000/ws"))
                .await
                .unwrap();
        assert!(socket_2
            .send(tungstenite::Message::text(&user_2_token))
            .await
            .is_ok());
        let user_3: login::LoginResponse = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_register(
                        &addr,
                        user::RegisterProfile {
                            name: "test_user_3".to_string(),
                            alias: None,
                            phone: "18999990003".to_string(),
                            gender: Some(1),
                            bio: None,
                            link: None,
                            password: "123456".to_string(),
                            email: "test_3@example.com".to_string(),
                        },
                    ))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        let user_3_token = user_3.token;
        let user_3 = jwt::JWTPayload::try_from(user_3_token.as_str()).unwrap().id;
        let (mut socket_3, _response) =
            tokio_tungstenite::connect_async(format!("ws://127.0.0.1:8000/ws"))
                .await
                .unwrap();
        assert!(socket_3
            .send(tungstenite::Message::text(&user_3_token))
            .await
            .is_ok());
        let task = tokio::task::spawn(async move {
            let feed_2: feed::Notification = match socket_2.next().await.unwrap().unwrap() {
                tungstenite::Message::Text(msg) => serde_json::from_str(&msg).unwrap(),
                _ => panic!("unexpected message"),
            };
            let feed_2 = match feed_2 {
                feed::Notification::ContactRequests { items } => items,
                _ => panic!("unexpected message"),
            };
            assert_eq!(feed_2.num, 1);
            assert_eq!(feed_2.items[0].id, user_1);
        });
        let response = client
            .request(request_add_contact(&addr, &user_1_token, user_2))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        task.await.unwrap();
        let response: contact::ContactList = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_get_new_contacts(&addr, &user_2_token))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert_eq!(response.num, 1);
        assert_eq!(response.items[0].id, user_1);
        let response = client
            .request(request_add_contact(&addr, &user_3_token, user_1))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = client
            .request(request_add_contact(&addr, &user_3_token, user_3))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = client
            .request(request_reject_contact(&addr, &user_1_token, user_3))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response = client
            .request(request_accept_contact(&addr, &user_2_token, user_1))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response: contact::ContactList = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_get_contacts(&addr, &user_2_token))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert_eq!(response.num, 1);
        let response: contact::ContactList = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_get_new_contacts(&addr, &user_2_token))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert_eq!(response.num, 0);
    }
}
