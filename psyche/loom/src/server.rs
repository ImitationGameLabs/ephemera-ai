use axum::Router;
use dotenv::dotenv;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::services::db_migration::Migrator;
use crate::services::memory::{
    manager::HybridMemoryManager,
    manager::MysqlMemoryManager,
};

use crate::services::memory::{handlers::MemoryHandler, AppState as MemoryAppState};
use crate::services::system_configs::{handlers::SystemConfigHandler, AppState as SystemConfigsAppState, manager::SystemConfigManager};

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
}

impl ServerConfig {
    /// Get the full bind address in [::]:port format for IPv6 compatibility
    pub fn bind_address(&self) -> String {
        format!("[::]:{}", self.port)
    }
}

/// HTTP server for the Loom memory service
pub struct LoomServer {
    config: ServerConfig,
    memory_manager: Arc<HybridMemoryManager>,
    system_config_manager: Arc<SystemConfigManager>,
}

impl LoomServer {
    /// Create a new server instance
    pub async fn new(config: ServerConfig) -> anyhow::Result<Self> {
        // Load environment variables
        dotenv().ok();

        // Initialize tracing
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "loom=debug,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        info!("Initializing Loom memory server");

        // Initialize database connection
        let conn = init_db_connection().await?;

        // Run migrations first
        run_migrations(&conn).await?;

        // Initialize memory manager and system config manager
        let memory_manager = Arc::new(init_memory_service(conn.clone()).await?);
        let system_config_manager = Arc::new(init_system_configs_service(conn).await?);

        Ok(Self {
            config,
            memory_manager,
            system_config_manager,
        })
    }

    /// Start the server
    pub async fn run(self) -> anyhow::Result<()> {
        use axum::routing::{get, post, delete};
        use tower_http::{
            cors::{Any, CorsLayer},
            trace::TraceLayer,
        };

        // Create app states for each service
        let memory_app_state = MemoryAppState {
            memory_manager: self.memory_manager.clone(),
        };
        let system_configs_app_state = SystemConfigsAppState {
            system_config_manager: self.system_config_manager.clone(),
        };

        let app = Router::new()
            .route("/health", get(crate::services::memory::health_check))
            .nest("/api/v1/memory",
                Router::new()
                    .route("/", post(MemoryHandler::create_memory))
                    .route("/", get(MemoryHandler::search_memory))
                    .route("/{id}", get(MemoryHandler::get_memory))
                    .route("/{id}", delete(MemoryHandler::delete_memory))
                    .with_state(memory_app_state)
            )
            .nest("/api/v1/system-configs",
                Router::new()
                    .route("/", post(SystemConfigHandler::create_system_config))
                    .route("/", get(SystemConfigHandler::query_system_configs))
                    .route("/{id}", get(SystemConfigHandler::get_system_config))
                    .with_state(system_configs_app_state)
            )
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
            .layer(TraceLayer::new_for_http());

        let bind_address = self.config.bind_address();
        let addr = format!("[::]:{}", self.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

        info!("Starting Loom server on {}", bind_address);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bind to address: {}", e))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}

async fn init_db_connection() -> anyhow::Result<DatabaseConnection> {
    let mysql_url = std::env::var("PSYCHE_LOOM_MYSQL_URL")
        .expect("PSYCHE_LOOM_MYSQL_URL not set");
    Database::connect(&mysql_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))
}

async fn init_memory_service(conn: DatabaseConnection) -> anyhow::Result<HybridMemoryManager> {
    Ok(HybridMemoryManager::new(MysqlMemoryManager::new(conn)))
}

async fn init_system_configs_service(conn: DatabaseConnection) -> anyhow::Result<SystemConfigManager> {
    Ok(SystemConfigManager::new(conn))
}

async fn run_migrations(conn: &DatabaseConnection) -> anyhow::Result<()> {
    info!("Running database migrations...");
    Migrator::up(conn, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
    info!("Database migrations completed!");

    Ok(())
}
