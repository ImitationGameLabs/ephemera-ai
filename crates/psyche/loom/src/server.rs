use axum::Router;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::services::memory::{
    AppState as MemoryAppState,
    handlers::{MemoryHandler, PinnedMemoryHandler},
    manager::MemoryManager,
};

/// HTTP server for the Loom memory service
pub struct LoomServer {
    config: Config,
    memory_manager: Arc<MemoryManager>,
}

impl LoomServer {
    /// Create a new server instance
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        // Initialize tracing
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "loom=info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        info!("Initializing Loom memory server");

        // Initialize memory manager with MySQL
        let memory_manager = Arc::new(init_memory_service(&config).await?);

        Ok(Self { config, memory_manager })
    }

    /// Start the server
    pub async fn run(self) -> anyhow::Result<()> {
        use axum::routing::{delete, get, post};
        use tower_http::{
            cors::{Any, CorsLayer},
            trace::TraceLayer,
        };

        // Create app state
        let memory_app_state = MemoryAppState { memory_manager: self.memory_manager.clone() };

        let app = Router::new()
            .route("/health", get(crate::services::memory::health_check))
            .nest(
                "/api/v1/memories",
                Router::new()
                    .route("/", post(MemoryHandler::create_memory))
                    .route("/views/recent", get(MemoryHandler::get_recent))
                    .route("/views/timeline", get(MemoryHandler::get_timeline))
                    .route("/{id}", get(MemoryHandler::get_memory))
                    .route("/{id}", delete(MemoryHandler::delete_memory))
                    .with_state(memory_app_state.clone()),
            )
            .nest(
                "/api/v1/pinned-memories",
                Router::new()
                    .route("/", get(PinnedMemoryHandler::get_pinned))
                    .route("/", post(PinnedMemoryHandler::pin_memory))
                    .route("/{memory_id}", delete(PinnedMemoryHandler::unpin_memory))
                    .with_state(memory_app_state),
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

        axum::serve(listener, app).await.map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}

async fn init_memory_service(config: &Config) -> anyhow::Result<MemoryManager> {
    let db = connect_db(config).await?;
    crate::services::db_migration::Migrator::up(&db, None).await?;
    Ok(MemoryManager::new(db, 0))
}

async fn connect_db(config: &Config) -> anyhow::Result<DatabaseConnection> {
    let mut db_options = sea_orm::ConnectOptions::new(config.mysql.url.clone());

    if let Some(max_conn) = config.mysql.max_connections {
        db_options.max_connections(max_conn);
    }

    let db = Database::connect(db_options).await?;
    info!("Connected to MySQL database");
    Ok(db)
}
