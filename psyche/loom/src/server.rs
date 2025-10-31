use dotenv::dotenv;
use qdrant_client::config::QdrantConfig;
use rig::providers::openai;
use rig::client::embeddings::EmbeddingsClientDyn;
use sea_orm_migration::MigratorTrait;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::services::memory::{
    manager::HybridMemoryManager,
    manager::{MysqlMemoryManager, QdrantMemoryManager},
    migration::Migrator,
};

use crate::{
    services::memory::AppState,
    services::memory::routes::create_routes,
};

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

        // Initialize memory manager
        let memory_manager = Arc::new(init_memory_manager().await?);

        Ok(Self {
            config,
            memory_manager,
        })
    }

    /// Start the server
    pub async fn run(self) -> anyhow::Result<()> {
        let app_state = AppState {
            memory_manager: self.memory_manager.clone(),
        };

        let app = create_routes(app_state);

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

async fn init_memory_manager() -> anyhow::Result<HybridMemoryManager> {
    // Setup MySQL connection
    let mysql_url = std::env::var("EPHA_MEMORY_MYSQL_URL")
        .expect("EPHA_MEMORY_MYSQL_URL not set");
    let conn = sea_orm::Database::connect(&mysql_url).await?;

    // Run database migrations
    info!("Running database migrations...");
    Migrator::up(&conn, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
    info!("Migrations completed successfully!");

    // Setup Qdrant connection
    let qdrant_url = std::env::var("EPHA_MEMORY_QDRANT_URL")
        .expect("EPHA_MEMORY_QDRANT_URL not set");
    let qdrant_config = QdrantConfig {
        uri: qdrant_url.clone(),
        ..Default::default()
    };
    let qdrant_client = qdrant_client::Qdrant::new(qdrant_config)
        .expect("Failed to create Qdrant client");

    // Initialize embedding model
    let embedding_model_name = std::env::var("EMBEDDING_MODEL")
        .expect("EMBEDDING_MODEL not set");
    let embedding_api_key = std::env::var("EMBEDDING_MODEL_API_KEY")
        .expect("EMBEDDING_MODEL_API_KEY not set");
    let embedding_url = std::env::var("EMBEDDING_MODEL_URL")
        .expect("EMBEDDING_MODEL_URL not set");

    // Create OpenAI-compatible client for custom embedding service
    let embedding_client = openai::Client::builder(&embedding_api_key)
        .base_url(&embedding_url)
        .build();

    // Get embedding dimensions (required)
    let embedding_dimensions: usize = std::env::var("EMBEDDING_MODEL_DIMENSIONS")
        .expect("EMBEDDING_MODEL_DIMENSIONS not set")
        .parse()
        .expect("EMBEDDING_MODEL_DIMENSIONS must be a valid number");

    let embedding_model = embedding_client.embedding_model(&embedding_model_name);

    let memory_manager = HybridMemoryManager::new(
        MysqlMemoryManager::new(conn),
        QdrantMemoryManager::new(qdrant_client, embedding_dimensions),
        embedding_model,
    );

    Ok(memory_manager)
}