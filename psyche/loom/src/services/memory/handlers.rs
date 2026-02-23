use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use tracing::{error, info, instrument};

use crate::memory::models::{
    ApiResponse, CreateMemoryRequest, MemoryResponse, RecentMemoryRequest, TimelineMemoryRequest,
};
use crate::services::memory::AppState;
use crate::services::memory::manager::Manager;

/// HTTP handler for memory operations
pub struct MemoryHandler;

impl MemoryHandler {
    /// Create memory fragments (supports batch operations)
    #[instrument(skip(state))]
    pub async fn create_memory(
        State(state): State<AppState>,
        Json(request): Json<CreateMemoryRequest>,
    ) -> Result<Json<ApiResponse<MemoryResponse>>, StatusCode> {
        info!("Creating {} memory fragments", request.fragments.len());

        if request.fragments.is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }

        // Set server-side timestamps for all fragments (overriding client timestamps)
        let now = time::OffsetDateTime::now_utc();
        let mut fragments = request.fragments;
        for fragment in &mut fragments {
            fragment.timestamp = now;
        }

        match state.memory_manager.append(&mut fragments).await {
            Ok(ids) => {
                info!("Successfully created {} memory fragments", ids.len());

                // Update fragments with their database-generated IDs
                for (fragment, id) in fragments.iter_mut().zip(ids) {
                    fragment.id = id;
                }

                let response = MemoryResponse::multiple(fragments);
                Ok(Json(ApiResponse::success(response)))
            }
            Err(e) => {
                error!("Failed to create memory fragments: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Get a specific memory fragment by ID
    #[instrument(skip(state))]
    pub async fn get_memory(
        State(state): State<AppState>,
        Path(id): Path<i64>,
    ) -> Result<Json<ApiResponse<MemoryResponse>>, StatusCode> {
        info!("Getting memory fragment with ID: {}", id);

        match state.memory_manager.get(id).await {
            Ok(memory_fragment) => {
                info!("Successfully retrieved memory fragment with ID: {}", id);
                let response = MemoryResponse::single(memory_fragment);
                Ok(Json(ApiResponse::success(response)))
            }
            Err(e) => {
                error!("Failed to get memory fragment with ID {}: {}", id, e);
                Err(StatusCode::NOT_FOUND)
            }
        }
    }

    /// Delete a memory fragment by ID
    #[instrument(skip(state))]
    pub async fn delete_memory(
        State(state): State<AppState>,
        Path(id): Path<i64>,
    ) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
        info!("Deleting memory fragment with ID: {}", id);

        match state.memory_manager.delete(id).await {
            Ok(()) => {
                info!("Successfully deleted memory fragment with ID: {}", id);
                Ok(Json(ApiResponse::success(
                    serde_json::json!({"deleted": true}),
                )))
            }
            Err(e) => {
                error!("Failed to delete memory fragment with ID {}: {}", id, e);
                Err(StatusCode::NOT_FOUND)
            }
        }
    }

    /// Get recent memory fragments
    #[instrument(skip(state))]
    pub async fn get_recent(
        State(state): State<AppState>,
        Query(request): Query<RecentMemoryRequest>,
    ) -> Result<Json<ApiResponse<MemoryResponse>>, StatusCode> {
        info!("Getting {} most recent memory fragments", request.limit);

        match state.memory_manager.get_recent(request.limit).await {
            Ok(fragments) => {
                info!("Successfully retrieved {} recent memory fragments", fragments.len());
                let response = MemoryResponse::multiple(fragments);
                Ok(Json(ApiResponse::success(response)))
            }
            Err(e) => {
                error!("Failed to get recent memory fragments: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Get memory fragments within a time range (timeline view)
    #[instrument(skip(state))]
    pub async fn get_timeline(
        State(state): State<AppState>,
        Query(request): Query<TimelineMemoryRequest>,
    ) -> Result<Json<ApiResponse<MemoryResponse>>, StatusCode> {
        info!(
            "Getting memory fragments from {} to {}",
            request.from, request.to
        );

        // Parse ISO 8601 time strings
        let time_range = request.parse().map_err(|e| {
            error!("Failed to parse time range: {}", e);
            StatusCode::BAD_REQUEST
        })?;

        match state
            .memory_manager
            .get_range(
                time_range.start,
                time_range.end,
                request.limit,
                request.offset,
            )
            .await
        {
            Ok(fragments) => {
                info!(
                    "Successfully retrieved {} memory fragments in time range",
                    fragments.len()
                );
                let response = MemoryResponse::multiple(fragments);
                Ok(Json(ApiResponse::success(response)))
            }
            Err(e) => {
                error!("Failed to get memory fragments in time range: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
