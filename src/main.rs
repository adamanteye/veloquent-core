#![warn(missing_docs)]
#![cfg_attr(all(coverage_nightly, test), feature(coverage_attribute))]

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
use param::Args;
use view::AppState;

async fn app(config: Config) -> Result<()> {
    event!(
        Level::INFO,
        "set upload directory to {:?}",
        &config.upload.dir
    );
    std::fs::create_dir_all(&config.upload.dir)?;
    let mut opt = ConnectOptions::new(format!(
        "postgres://{}:{}@{}:{}/{}",
        config.database.username,
        config.database.password,
        config.database.address,
        config.database.port,
        config.database.name,
    ));
    opt.max_connections(config.database.max_connections);
    event!(Level::INFO, "connecting to database");
    let db: DatabaseConnection = Database::connect(opt).await?;
    event!(
        Level::INFO,
        "connected to database {} at {}:{} as {}",
        config.database.name,
        config.database.address,
        config.database.port,
        config.database.username
    );
    #[cfg(feature = "dev")]
    {
        use migration::{Migrator, MigratorTrait};
        use sea_orm::ConnectionTrait;
        db.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";".to_owned(),
        ))
        .await?;
        Migrator::up(&db, None).await?;
        event!(Level::WARN, "migrated database");
    }
    #[cfg(test)]
    {
        use migration::MigratorTrait;
        use sea_orm::ConnectionTrait;
        migration::Migrator::down(&db, Some(8)).await.unwrap();
        db.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";".to_owned(),
        ))
        .await
        .unwrap();
        migration::Migrator::up(&db, None).await.unwrap();
    }
    let secret = config.authentication.secret;
    jwt::JWT_SETTING.get_or_init(|| jwt::JwtSetting {
        exp: config.authentication.exp_after,
        de_key: jwt::DecodingKey::from_secret(secret.as_bytes()),
        en_key: jwt::EncodingKey::from_secret(secret.as_bytes()),
    });
    utility::UPLOAD_DIR.get_or_init(|| config.upload.dir);
    let state = AppState {
        conn: db,
        ws_pool: Default::default(),
    };
    let app = view::router(state);
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.listen.address, config.listen.port))
            .await?;
    event!(
        Level::INFO,
        "listen on http://{}:{}",
        config.listen.address,
        config.listen.port
    );
    #[cfg(feature = "dev")]
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

#[doc(hidden)]
#[instrument(name = "veloquent_main")]
#[tokio::main]
async fn main() -> Result<()> {
    // see https://stackoverflow.com/questions/73247589/how-to-turn-off-tracing-events-emitted-by-other-crates
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::WARN.into())
        .from_env()?
        .add_directive("veloquent_core=debug".parse()?);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();
    let args = Args::parse();
    let config_path = std::path::Path::new(args.config.as_str());
    event!(Level::WARN, "reading configuration from {:?}", config_path);
    let config = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(config.as_str())?;
    app(config).await
}
