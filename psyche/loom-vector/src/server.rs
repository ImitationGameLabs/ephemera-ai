use axum::Router;
use dotenv::dotenv;
use qdrant_client::config::QdrantConfig;
use rig::providers::openai;
use rig::client::embeddings::EmbeddingsClientDyn;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::services::memory::{
    manager::{VectorSearchManager, QdrantMemoryManager},
};

use crate::services::memory::{handlers::MemoryHandler, AppState as MemoryAppState};

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

/// HTTP server for the Loom Vector Search service
pub struct LoomVectorServer {
    config: ServerConfig,
    vector_search_manager: Arc<VectorSearchManager>,
}

impl LoomVectorServer {
    /// Create a new server instance
    pub async fn new(config: ServerConfig) -> anyhow::Result<Self> {
        // Load environment variables
        dotenv().ok();

        // Initialize tracing
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "loom_vector=debug,tower_http=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        info!("Initializing Loom Vector Search server");

        // Initialize vector search manager
        let vector_search_manager = Arc::new(init_vector_search_service().await?);

        Ok(Self {
            config,
            vector_search_manager,
        })
    }

    /// Start the server
    pub async fn run(self) -> anyhow::Result<()> {
        use axum::routing::{get, post, delete};
        use tower_http::{
            cors::{Any, CorsLayer},
            trace::TraceLayer,
        };

        // Create app state
        let app_state = MemoryAppState {
            vector_search_manager: self.vector_search_manager.clone(),
        };

        let app = Router::new()
            .route("/health", get(crate::services::memory::health_check))
            // TODO: Add vector search API endpoints
            // .route("/api/v1/vector/index", post(...))
            // .route("/api/v1/vector/search", get(...))
            // .route("/api/v1/vector/{id}", delete(...))
            .nest("/api/v1/memory",
                Router::new()
                    .route("/", post(MemoryHandler::create_memory))
                    .route("/", get(MemoryHandler::search_memory))
                    .route("/{id}", delete(MemoryHandler::delete_memory))
                    .with_state(app_state)
            )
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
            .layer(TraceLayer::new_for_http());

        let bind_address = self.config.bind_address();
        let addr = format!("[::]:{}", self.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

        info!("Starting Loom Vector server on {}", bind_address);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bind to address: {}", e))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}

async fn init_vector_search_service() -> anyhow::Result<VectorSearchManager> {
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

    let vector_search_manager = VectorSearchManager::new(
        QdrantMemoryManager::new(qdrant_client, embedding_dimensions),
        embedding_model,
    );

    Ok(vector_search_manager)
}
