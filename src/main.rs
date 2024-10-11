#![warn(missing_docs)]

//! Voloquent 后端服务
//!
//! 队名 Veloquent 结合拉丁语 `velox`(快速) 和 `eloquent` (雄辩), 表达快速而清晰的沟通能力.

use anyhow::Result;
use axum::Router;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod config;
#[doc(hidden)]
mod param;
#[doc(hidden)]
mod utility;

use config::Config;

#[doc(hidden)]
#[derive(OpenApi)]
#[openapi()]
struct ApiDoc;

#[doc(hidden)]
#[derive(Clone)]
struct AppState {
    conn: DatabaseConnection,
}

#[doc(hidden)]
#[tokio::main]
async fn main() -> Result<()> {
    let config = std::fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(config.as_str())?;
    // Postgres database
    let mut opt = ConnectOptions::new(format!(
        "postgres://{}:{}@{}:{}/{}",
        config.database.username,
        config.database.password,
        config.database.host,
        config.database.port,
        config.database.name,
    ));
    opt.max_connections(config.database.max_connections);
    let db: DatabaseConnection = Database::connect(opt).await?;
    let state = AppState { conn: db };
    let app = Router::new()
        .merge(SwaggerUi::new("/doc").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(utility::shutdown_signal())
        .await?;
    Ok(())
}
