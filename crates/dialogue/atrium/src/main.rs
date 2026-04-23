mod config;
mod db;
mod entity;
mod handlers;
mod migration;
mod models;
mod routes;

use clap::Parser;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::path::PathBuf;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::db::{message_manager::MessageManager, user_manager::UserManager};
use crate::migration::Migrator;
use crate::routes::create_routes;

#[derive(Parser)]
#[command(name = "atrium")]
struct Args {
    /// Directory containing config files
    #[arg(long, default_value = ".config")]
    config_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config_path = args.config_dir.join("atrium.json");
    let config = Config::load(&config_path);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "atrium=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Connect to database
    let mut attempt = 0u32;
    let mut delay = Duration::from_secs(1);
    let conn: DatabaseConnection = loop {
        match Database::connect(&config.mysql_url).await {
            Ok(db) => {
                tracing::info!("Connected to database");
                break db;
            }
            Err(e) => {
                attempt += 1;
                tracing::warn!(
                    "MySQL connection failed (attempt {attempt}): {e}. Retrying in {}s...",
                    delay.as_secs()
                );
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(30));
            }
        }
    };

    // Run migrations
    Migrator::up(&conn, None).await?;
    tracing::info!("Database migrations completed");

    // Create separate managers with their own connections
    let user_conn: DatabaseConnection = Database::connect(&config.mysql_url).await?;
    let message_conn: DatabaseConnection = Database::connect(&config.mysql_url).await?;

    let user_manager = UserManager::new(user_conn);
    let message_manager = MessageManager::new(message_conn);

    // Create axum app
    let app = create_routes(user_manager, message_manager);

    // Start server
    let bind_address = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!("Atrium service listening on {}", bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}
