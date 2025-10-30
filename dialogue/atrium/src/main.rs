mod db;
mod entity;
mod handlers;
mod migration;
mod models;
mod routes;

use dotenv::dotenv;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt
};
use std::env;

use crate::db::{
    user_manager::UserManager, 
    message_manager::MessageManager,
};
use crate::migration::Migrator;
use crate::routes::create_routes;

/// Get service port from environment variable with default
fn get_service_port() -> u16 {
    env::var("ATRIUM_SERVICE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3002)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get database URL from environment variable
    let database_url = env::var("DIALOGUE_ATRIUM_MYSQL_URL")
        .expect("DIALOGUE_ATRIUM_MYSQL_URL must be set");

    // Connect to database
    let conn: DatabaseConnection = Database::connect(&database_url).await?;
    tracing::info!("Connected to database");

    // Run migrations
    Migrator::up(&conn, None).await?;
    tracing::info!("Database migrations completed");

    // Create separate managers for users and messages with their own connections
    let user_conn: DatabaseConnection = Database::connect(&database_url).await?;
    let message_conn: DatabaseConnection = Database::connect(&database_url).await?;

    let user_manager = UserManager::new(user_conn);
    let message_manager = MessageManager::new(message_conn);

    // Get service port from environment variable
    let port = get_service_port();
    let bind_address = format!("[::]:{}", port);

    // Create axum app
    let app = create_routes(user_manager, message_manager);

    // Start server
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    tracing::info!("Atrium service listening on {}", bind_address);

    axum::serve(listener, app).await?;

    Ok(())
}