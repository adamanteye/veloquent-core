#![warn(missing_docs)]

//! Voloquent 后端服务
//!
//! 队名 *Veloquent* 结合拉丁语 _velox_(快速) 和 _eloquent_ (雄辩), 表达快速而清晰的沟通能力.

use anyhow::Result;
use axum::Router;
use clap::Parser;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::{event, instrument, Level};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod config;
#[doc(hidden)]
mod entity;
pub mod param;
#[doc(hidden)]
mod utility;

use config::Config;
use migration::{Migrator, MigratorTrait};
use param::Args;

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
#[instrument(name = "volequent_main")]
#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .pretty()
        .with_file(true)
        .with_line_number(false)
        .with_thread_ids(true)
        .with_target(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    let args = Args::parse();
    let config_path = std::path::Path::new(args.config.as_str());
    event!(Level::WARN, "reading configuration from {:?}", config_path);
    let config = std::fs::read_to_string(config_path)?;
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
    event!(
        Level::INFO,
        "connected to database {} at {}:{} as {}",
        config.database.name,
        config.database.host,
        config.database.port,
        config.database.username
    );
    Migrator::up(&db, None).await?;

    event!(Level::WARN, "Migrated database");
    let state = AppState { conn: db };
    let app = Router::new()
        .merge(SwaggerUi::new("/doc").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state);
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.listen.address, config.listen.port))
            .await?;
    event!(
        Level::INFO,
        "listening on {}:{}",
        config.listen.address,
        config.listen.port
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(utility::shutdown_signal())
        .await?;
    Ok(())
}
