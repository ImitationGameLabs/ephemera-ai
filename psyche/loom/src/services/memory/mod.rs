pub mod entity;
pub mod handlers;
pub mod manager;
pub mod migration;
pub mod routes;

use axum::{
    http::StatusCode,
    response::Json,
};
use serde_json::Value;
use std::sync::Arc;
use time::OffsetDateTime;

use manager::HybridMemoryManager;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub memory_manager: Arc<HybridMemoryManager>,
}

/// Health check endpoint
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "ok",
        "service": "loom-memory-service",
        "timestamp": OffsetDateTime::now_utc().unix_timestamp()
    })))
}