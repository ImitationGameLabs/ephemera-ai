use dotenv::dotenv;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt
};
use std::env;

use dialogue_atrium::db::{UserManager, MessageManager};
use dialogue_atrium::migration::Migrator;
use dialogue_atrium::routes::create_routes;

const BIND_ADDRESS: &str = "127.0.0.1:3000";

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

    // Create axum app
    let app = create_routes(user_manager, message_manager);

    // Start server
    let listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;
    tracing::info!("Server listening on {}", BIND_ADDRESS);

    axum::serve(listener, app).await?;

    Ok(())
}