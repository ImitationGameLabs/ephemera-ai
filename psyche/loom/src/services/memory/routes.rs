use axum::{
    routing::{get, post, delete},
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

use crate::services::memory::{handlers::MemoryHandler, AppState, health_check};

/// Create application routes
pub fn create_routes(state: AppState) -> Router {
    Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // API v1 routes
        .nest("/api/v1", api_v1_routes().with_state(state))
        // Middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
}

/// API v1 routes
fn api_v1_routes() -> Router<AppState> {
    Router::new()
        .nest("/memory", memory_routes())
}

/// Memory management routes
fn memory_routes() -> Router<AppState> {
    Router::new()
        .route("/", post(MemoryHandler::create_memory))
        .route("/", get(MemoryHandler::search_memory))
        .route("/:id", get(MemoryHandler::get_memory))
        .route("/:id", delete(MemoryHandler::delete_memory))
}