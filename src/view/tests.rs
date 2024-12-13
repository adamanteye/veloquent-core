use super::*;

use crate::jwt;
use crate::utility;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sea_orm::{ConnectOptions, Database};

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

#[tokio::test]
async fn integration() {
    let addr = "127.0.0.1:8000";
    tokio::spawn(start_http_server(addr));
    let addr = format!("http://{addr}");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
        .build_http();
    let response = client
        .request(
            Request::builder()
                .uri(format!("{}/doc/", addr))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // test if swagger doc is available
    assert_eq!(response.status(), StatusCode::OK);
}
