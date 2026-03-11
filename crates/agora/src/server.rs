//! HTTP server for Agora event hub.

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::handlers::{EventHandler, HeraldHandler};
use crate::herald::HeraldRegistry;
use crate::queue::EventQueue;

/// Application state shared across handlers.
#[derive(Clone)]
pub struct AppState {
    pub event_queue: Arc<EventQueue>,
    pub herald_registry: Arc<HeraldRegistry>,
}

/// HTTP server for the Agora event hub.
pub struct AgoraServer {
    config: Config,
    state: Arc<AppState>,
}

impl AgoraServer {
    /// Creates a new server instance.
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        // Initialize tracing
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "agora=info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        info!("Initializing Agora event hub");

        let event_queue = Arc::new(
            EventQueue::new(&config.database_path, config.retry.clone()).await?,
        );
        let herald_registry = Arc::new(HeraldRegistry::new());

        let state = Arc::new(AppState {
            event_queue,
            herald_registry,
        });

        Ok(Self { config, state })
    }

    /// Starts the server.
    pub async fn run(self) -> anyhow::Result<()> {
        use axum::routing::{delete, get, patch, post};
        use tower_http::{
            cors::{Any, CorsLayer},
            trace::TraceLayer,
        };

        // Spawn background heartbeat timeout checker
        let herald_registry = self.state.herald_registry.clone();
        let check_interval = Duration::from_millis(self.config.heartbeat_check_interval_ms);
        let timeout_ms = self.config.timeout_ms;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(check_interval);
            loop {
                interval.tick().await;
                let changed = herald_registry.check_timeouts(timeout_ms).await;
                for (id, status) in changed {
                    info!("Herald '{}' status changed to {:?}", id, status);
                }
            }
        });

        let app = Router::new()
            .route("/health", get(health_check))
            // Herald routes
            .route("/heralds", post(HeraldHandler::register))
            .route("/heralds", get(HeraldHandler::list))
            .route("/heralds/{id}", get(HeraldHandler::get))
            .route("/heralds/{id}", delete(HeraldHandler::unregister))
            .route("/heralds/{id}/heartbeat", post(HeraldHandler::heartbeat))
            // Event routes
            .route("/events", post(EventHandler::create))
            .route("/events", patch(EventHandler::batch_update))
            .route("/events/fetch", post(EventHandler::fetch))
            .route("/events/{id}", patch(EventHandler::update))
            .with_state((*self.state).clone())
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
            .layer(TraceLayer::new_for_http());

        let bind_address = self.config.bind_address();
        let addr: SocketAddr = bind_address
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

        info!("Starting Agora server on {}", bind_address);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bind to address: {}", e))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "OK"
}
