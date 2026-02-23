use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::Value;
use tracing::{error, info, instrument};

use crate::memory::models::{
    ApiResponse, CreateMemoryRequest, MemoryResponse, SearchMemoryRequest,
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
            fragment.objective_metadata.created_at = now;
            fragment.objective_metadata.updated_at = now;
        }

        match state.vector_search_manager.append(&mut fragments).await {
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

    /// Search memory fragments
    #[instrument(skip(state))]
    pub async fn search_memory(
        State(state): State<AppState>,
        Query(request): Query<SearchMemoryRequest>,
    ) -> Result<Json<ApiResponse<MemoryResponse>>, StatusCode> {
        info!(
            "Searching memory fragments with keywords: {}",
            request.keywords
        );

        let query = request.into();

        match state.vector_search_manager.recall(&query).await {
            Ok(result) => {
                info!("Found {} memory fragments", result.memories.len());
                let response = MemoryResponse::from(result);
                Ok(Json(ApiResponse::success(response)))
            }
            Err(e) => {
                error!("Failed to search memory fragments: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Delete a memory fragment by ID
    #[instrument(skip(state))]
    pub async fn delete_memory(
        State(state): State<AppState>,
        Path(id): Path<i64>,
    ) -> Result<Json<ApiResponse<Value>>, StatusCode> {
        info!("Deleting memory fragment with ID: {}", id);

        match state.vector_search_manager.delete(id).await {
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
}
