#![warn(missing_docs)]

//! Veloquent 后端服务
//!
//! 队名 *Veloquent* 结合拉丁语 _velox_(快速) 和 _eloquent_ (雄辩), 表达快速而清晰的沟通能力.

use anyhow::Result;

use clap::Parser;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::{event, instrument, Level};

pub mod config;
#[doc(hidden)]
pub mod entity;
pub mod error;
pub mod jwt;
pub mod param;
#[doc(hidden)]
pub mod utility;
pub mod view;

use config::Config;
use migration::{Migrator, MigratorTrait};
use param::Args;
use view::AppState;

#[doc(hidden)]
#[instrument(name = "veloquent_main")]
#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(true)
        .with_line_number(false)
        .with_thread_ids(false)
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
        config.database.address,
        config.database.port,
        config.database.name,
    ));
    opt.max_connections(config.database.max_connections);
    let db: DatabaseConnection = Database::connect(opt).await?;
    event!(
        Level::INFO,
        "connected to database {} at {}:{} as {}",
        config.database.name,
        config.database.address,
        config.database.port,
        config.database.username
    );
    Migrator::up(&db, None).await?;
    event!(Level::WARN, "Migrated database");
    let secret = config.authentication.secret;
    jwt::JWT_SETTING
        .set(jwt::JwtSetting {
            exp: config.authentication.exp_after,
            de_key: jwt::DecodingKey::from_secret(secret.as_bytes()),
            en_key: jwt::EncodingKey::from_secret(secret.as_bytes()),
        })
        .map_err(|_| Err::<(), ()>(()))
        .unwrap();
    jwt::JWT_ALG
        .set(jsonwebtoken::Validation::new(
            jsonwebtoken::Algorithm::HS256,
        ))
        .map_err(|_| Err::<(), ()>(()))
        .unwrap();

    let state = AppState { conn: db };
    let app = view::router(state);
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.listen.address, config.listen.port))
            .await?;
    event!(
        Level::INFO,
        "listening on http://{}:{}",
        config.listen.address,
        config.listen.port
    );
    event!(
        Level::INFO,
        "serve doc on http://{}:{}{}",
        config.listen.address,
        config.listen.port,
        view::DOC_PATH
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(utility::shutdown_signal())
        .await?;
    Ok(())
}
