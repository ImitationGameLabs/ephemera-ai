pub mod handlers;
pub mod manager;

use axum::{
    http::StatusCode,
    response::Json,
};
use serde_json::Value;
use std::sync::Arc;
use time::OffsetDateTime;

use manager::VectorSearchManager;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub vector_search_manager: Arc<VectorSearchManager>,
}

/// Health check endpoint
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "ok",
        "service": "loom-vector-service",
        "timestamp": OffsetDateTime::now_utc().unix_timestamp()
    })))
}
