use super::*;

use super::user;
use crate::{jwt, utility};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseBackend};

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

fn request_doc(addr: &str) -> Request<Body> {
    Request::builder()
        .uri(format!("{addr}/doc/"))
        .body(Body::empty())
        .unwrap()
}

fn request_register(addr: &str) -> Request<Body> {
    Request::builder()
        .uri(format!("{addr}/register"))
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&user::RegisterProfile {
                name: "test_user".to_string(),
                alias: None,
                phone: "18999990000".to_string(),
                gender: Some(1),
                bio: None,
                link: None,
                password: "123456".to_string(),
                email: "test@example.com".to_string(),
            })
            .unwrap(),
        ))
        .unwrap()
}

#[tokio::test]
async fn integration() {
    let addr = "127.0.0.1:8000";
    tokio::spawn(start_http_server(addr));
    let addr = format!("http://{addr}");
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build_http();
    // test if swagger doc is available
    let response = client.request(request_doc(&addr)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    // test if register is available
    let response = client.request(request_register(&addr)).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}
