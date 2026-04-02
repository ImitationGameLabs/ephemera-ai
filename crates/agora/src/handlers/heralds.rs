//! Herald HTTP handlers.

use axum::extract::State;
use axum::{extract::Path, http::StatusCode, response::Json};
use tracing::{info, instrument};

use crate::herald::{HeartbeatResponse, HeraldInfo, HeraldsListResponse, RegisterHeraldRequest};
use crate::server::AppState;

/// HTTP handler for herald operations.
pub struct HeraldHandler;

impl HeraldHandler {
    /// Register a new herald (POST /heralds).
    #[instrument(skip(state))]
    pub async fn register(
        State(state): State<AppState>,
        Json(request): Json<RegisterHeraldRequest>,
    ) -> Result<Json<HeraldInfo>, StatusCode> {
        info!("Registering herald: {}", request.id);

        let info = state.herald_registry.register(request).await;
        info!("Registered herald: {}", info.id);
        Ok(Json(info))
    }

    /// List all heralds (GET /heralds).
    #[instrument(skip(state))]
    pub async fn list(State(state): State<AppState>) -> Json<HeraldsListResponse> {
        info!("Listing heralds");

        let heralds = state.herald_registry.list().await;
        info!("Found {} heralds", heralds.len());

        Json(HeraldsListResponse { heralds })
    }

    /// Get a single herald (GET /heralds/{id}).
    #[instrument(skip(state))]
    pub async fn get(
        State(state): State<AppState>,
        Path(id): Path<String>,
    ) -> Result<Json<HeraldInfo>, StatusCode> {
        info!("Getting herald: {}", id);

        match state.herald_registry.get(&id).await {
            Some(info) => {
                info!("Found herald: {}", id);
                Ok(Json(info))
            }
            None => {
                info!("Herald not found: {}", id);
                Err(StatusCode::NOT_FOUND)
            }
        }
    }

    /// Update herald heartbeat (POST /heralds/{id}/heartbeat).
    #[instrument(skip(state))]
    pub async fn heartbeat(
        State(state): State<AppState>,
        Path(id): Path<String>,
    ) -> Result<Json<HeartbeatResponse>, StatusCode> {
        info!("Herald heartbeat: {}", id);

        match state.herald_registry.heartbeat(&id).await {
            Some(response) => {
                info!("Updated heartbeat for herald: {}", id);
                Ok(Json(response))
            }
            None => {
                info!("Herald not found for heartbeat: {}", id);
                Err(StatusCode::NOT_FOUND)
            }
        }
    }

    /// Unregister a herald (DELETE /heralds/{id}).
    #[instrument(skip(state))]
    pub async fn unregister(
        State(state): State<AppState>,
        Path(id): Path<String>,
    ) -> Result<StatusCode, StatusCode> {
        info!("Unregistering herald: {}", id);

        if state.herald_registry.unregister(&id).await {
            info!("Unregistered herald: {}", id);
            Ok(StatusCode::NO_CONTENT)
        } else {
            info!("Herald not found for unregister: {}", id);
            Err(StatusCode::NOT_FOUND)
        }
    }
}
