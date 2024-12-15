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
            "/logout",
            delete(login::logout_handler).route_layer(auth.clone()),
        )
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

    fn request_login(addr: &str, req: login::LoginRequest) -> Request<Body> {
        request_post_json()
            .uri(format!("{addr}/login"))
            .body(Body::from(serde_json::to_vec(&req).unwrap()))
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

    fn request_get_contacts(addr: &str, token: &str, params: &str) -> Request<Body> {
        request_get_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/contact/list{params}"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_edit_user(
        addr: &str,
        token: &str,
        edition: user::UserProfileEdition,
    ) -> Request<Body> {
        request_put_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/user/profile"))
            .body(Body::from(serde_json::to_vec(&edition).unwrap()))
            .unwrap()
    }

    fn request_delete() -> http::request::Builder {
        Request::builder().method("DELETE")
    }

    fn request_delete_user(addr: &str, token: &str) -> Request<Body> {
        request_delete()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/user/profile"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_edit_contact(addr: &str, token: &str, id: Uuid, params: &str) -> Request<Body> {
        request_put_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/contact/edit/{id}{params}"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_get_categories(addr: &str, token: &str) -> Request<Body> {
        request_get_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/contact/categories"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_create_group(addr: &str, token: &str, group: group::GroupPost) -> Request<Body> {
        request_post_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/group/new"))
            .body(Body::from(serde_json::to_vec(&group).unwrap()))
            .unwrap()
    }

    fn request_group_invite(
        addr: &str,
        token: &str,
        group: Uuid,
        users: Vec<Uuid>,
    ) -> Request<Body> {
        request_post_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/group/invite/{group}"))
            .body(Body::from(serde_json::to_vec(&users).unwrap()))
            .unwrap()
    }

    fn request_group_view(addr: &str, token: &str, group: Uuid) -> Request<Body> {
        request_get_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/group/manage/{group}"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_group_manage(addr: &str, token: &str, group: Uuid, params: &str) -> Request<Body> {
        request_put_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/group/manage/{group}{params}"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_group_approve(addr: &str, token: &str, group: Uuid, params: &str) -> Request<Body> {
        request_put_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/group/approve/{group}{params}"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_send_msg(
        addr: &str,
        token: &str,
        session: Uuid,
        msg: super::message::MsgPost,
    ) -> Request<Body> {
        request_post_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/msg/session/{session}"))
            .body(Body::from(serde_json::to_vec(&msg).unwrap()))
            .unwrap()
    }

    fn request_get_msg(addr: &str, token: &str, session: Uuid) -> Request<Body> {
        request_get_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/msg/session/{session}"))
            .body(Body::empty())
            .unwrap()
    }

    fn request_renew_jwt(addr: &str, token: &str) -> Request<Body> {
        request_get_json()
            .header("Authorization", format!("Bearer {token}"))
            .uri(format!("{addr}/renew"))
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn integration() {
        let addr = "127.0.0.1:8000";
        tokio::spawn(start_http_server(addr));
        let ws_url = format!("ws://{addr}/ws");
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
        let response = client
            .request(request_login(
                &addr,
                login::LoginRequest {
                    name: "test_user_1".to_string(),
                    password: "123456".to_string(),
                },
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let token = response.into_body().collect().await.unwrap().aggregate();
        let token: login::LoginResponse = serde_json::from_reader(token.reader()).unwrap();
        let user_1_token = token.token;
        let user_1 = jwt::JWTPayload::try_from(user_1_token.as_str()).unwrap().id;
        let (mut socket_1, _response) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
        assert!(socket_1
            .send(tungstenite::Message::text(&user_1_token))
            .await
            .is_ok());
        let socket_1 = Arc::new(Mutex::new(socket_1));
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
        let (mut socket_2, _response) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
        assert!(socket_2
            .send(tungstenite::Message::text(&user_2_token))
            .await
            .is_ok());
        let socket_2 = Arc::new(Mutex::new(socket_2));
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
        let (mut socket_3, _response) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();
        assert!(socket_3
            .send(tungstenite::Message::text(&user_3_token))
            .await
            .is_ok());
        let socket_3 = Arc::new(Mutex::new(socket_3));
        let task = tokio::task::spawn(async move {
            let feed_2: feed::Notification =
                match socket_2.lock().await.next().await.unwrap().unwrap() {
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
        let socket = socket_1.clone();
        let task = tokio::task::spawn(async move {
            let feed: feed::Notification = match socket.lock().await.next().await.unwrap().unwrap()
            {
                tungstenite::Message::Text(msg) => serde_json::from_str(&msg).unwrap(),
                _ => panic!("unexpected message"),
            };
            let feed = match feed {
                feed::Notification::ContactRequests { items } => items,
                _ => panic!("unexpected message"),
            };
            assert_eq!(feed.num, 1);
            assert_eq!(feed.items[0].id, user_3);
        });
        let response = client
            .request(request_add_contact(&addr, &user_3_token, user_1))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        task.await.unwrap();
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
        let socket = socket_1.clone();
        let task = tokio::task::spawn(async move {
            let feed: feed::Notification = match socket.lock().await.next().await.unwrap().unwrap()
            {
                tungstenite::Message::Text(msg) => serde_json::from_str(&msg).unwrap(),
                _ => panic!("unexpected message"),
            };
            let feed = match feed {
                feed::Notification::ContactAccepts { items } => items,
                _ => panic!("unexpected message"),
            };
            assert_eq!(feed.num, 1);
            assert_eq!(feed.items[0].id, user_2);
        });
        let response = client
            .request(request_accept_contact(&addr, &user_2_token, user_1))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        task.await.unwrap();
        let response: contact::ContactList = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_get_contacts(&addr, &user_2_token, ""))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert_eq!(response.num, 1);
        let chat_1_2 = response.items[0].session;
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
        // user_1 and user_2 are now friends
        let consume_msg = |socket: Arc<
            tokio::sync::Mutex<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
            >,
        >| async move {
            let _ = socket.lock().await.next().await.unwrap();
        };
        let response = client
            .request(request_add_contact(&addr, &user_1_token, user_3))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        consume_msg(socket_3).await;
        let response = client
            .request(request_accept_contact(&addr, &user_3_token, user_1))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        consume_msg(socket_1.clone()).await;
        // user_1 and user_3 are now contacts
        // test if user can be deleted
        let user: login::LoginResponse = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_register(
                        &addr,
                        user::RegisterProfile {
                            name: "test_user1".to_string(),
                            alias: None,
                            phone: "18999990004".to_string(),
                            gender: Some(1),
                            bio: None,
                            link: None,
                            password: "123456".to_string(),
                            email: "test1@example.com".to_string(),
                        },
                    ))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        let user_token = user.token;
        let response = client
            .request(request_delete_user(&addr, &user_token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        // test if user profile can be edited
        let response = client
            .request(request_edit_user(
                &addr,
                &user_1_token,
                user::UserProfileEdition {
                    name: Some("test_user1".to_string()),
                    alias: Some("monika".to_string()),
                    bio: Some("test_bio".to_string()),
                    link: None,
                    phone: None,
                    email: None,
                    gender: None,
                },
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // test if contact can be edited
        let response = client
            .request(request_edit_contact(
                &addr,
                &user_1_token,
                user_2,
                "?category=family",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // test if group can be created
        let group: group::GroupProfile = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_create_group(
                        &addr,
                        &user_1_token,
                        group::GroupPost {
                            name: Some("test_group".to_string()),
                            members: vec![user_1, user_2],
                        },
                    ))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert!(group.members.contains(&user_1));
        assert!(group.members.contains(&user_2));
        assert_eq!(group.owner, user_1);
        // test if group member can invite users
        // test if group owner can receive notification
        let socket = socket_1.clone();
        let task = tokio::task::spawn(async move {
            let feed: feed::Notification = match socket.lock().await.next().await.unwrap().unwrap()
            {
                tungstenite::Message::Text(msg) => serde_json::from_str(&msg).unwrap(),
                _ => panic!("unexpected message"),
            };
            let feed = match feed {
                feed::Notification::GroupRequests { items } => items,
                _ => panic!("unexpected message"),
            };
            assert_eq!(feed[0].group, group.id);
            assert_eq!(feed[0].user, user_3);
        });
        let response = client
            .request(request_group_invite(
                &addr,
                &user_1_token,
                group.id,
                vec![user_3],
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        task.await.unwrap();
        // test if group owner can manage admin
        let response = client
            .request(request_group_manage(
                &addr,
                &user_1_token,
                group.id,
                &format!("?admin={user_2}"),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response: Vec<Uuid> = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_group_view(&addr, &user_1_token, group.id))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert!(!response.contains(&user_2));
        assert!(!response.contains(&user_1));
        assert!(response.contains(&user_3));
        // test if group admin can approve invitation
        let response = client
            .request(request_group_approve(
                &addr,
                &user_2_token,
                group.id,
                &format!("?member={user_3}"),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // test if group owner can be transferred
        let response = client
            .request(request_group_manage(
                &addr,
                &user_1_token,
                group.id,
                &format!("?owner={user_2}"),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        // test if user can view their categories
        let response: Vec<String> = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_get_categories(&addr, &user_1_token))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert_eq!(response.len(), 1);
        assert_eq!(response[0], "family");
        // test if user can send message to contact
        let response = client
            .request(request_send_msg(
                &addr,
                &user_1_token,
                chat_1_2,
                super::message::MsgPost {
                    content: Some("Hello, world".to_string()),
                    typ: 0,
                    cite: None,
                    file: None,
                    forward: None,
                    notice: None,
                },
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        // test if user can get message from contact
        let history: history::History = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_get_msg(&addr, &user_2_token, chat_1_2))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert_eq!(history.msgs.len(), 1);
        assert_eq!(history.start, 0);
        assert_eq!(history.end, 1);
        assert_eq!(history.cnt, 1);
        assert_eq!(history.msgs[0].content, Some("Hello, world".to_string()));
        // test if user can send message to group
        let response = client
            .request(request_send_msg(
                &addr,
                &user_1_token,
                group.session,
                super::message::MsgPost {
                    content: Some("Hallo, Welt!".to_string()),
                    typ: 0,
                    cite: None,
                    file: None,
                    forward: None,
                    notice: None,
                },
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        // test if user can get message from group
        let history: history::History = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_get_msg(&addr, &user_3_token, group.session))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert_eq!(history.msgs.len(), 1);
        assert_eq!(history.msgs[0].content, Some("Hallo, Welt!".to_string()));
        // test if user can renew jwt
        let response: login::LoginResponse = serde_json::from_reader(
            res_to_json(
                client
                    .request(request_renew_jwt(&addr, &user_1_token))
                    .await
                    .unwrap(),
            )
            .await,
        )
        .unwrap();
        assert_ne!(response.token, user_1_token);
    }
}
