use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::Value;
use time::OffsetDateTime;
use tracing::{error, info, instrument};

use crate::memory::{
    types::{MemoryFragment, MemorySource, SubjectiveMetadata, ObjectiveMetadata},
    manager::Manager,
    models::{ApiResponse, CreateMemoryRequest, MemoryResponse, SearchMemoryRequest, SearchMemoryResponse}
};
use crate::services::memory::AppState;

/// HTTP handler for memory operations
pub struct MemoryHandler;

impl MemoryHandler {
    /// Create a new memory fragment
    #[instrument(skip(state))]
    pub async fn create_memory(
        State(state): State<AppState>,
        Json(request): Json<CreateMemoryRequest>,
    ) -> Result<Json<ApiResponse<MemoryResponse>>, StatusCode> {
        info!("Creating new memory fragment");

        let now = OffsetDateTime::now_utc().unix_timestamp();
        let memory_fragment = MemoryFragment {
            id: 0, // Will be set by database
            content: request.content.clone(),
            subjective_metadata: SubjectiveMetadata {
                importance: 100, // Default importance
                confidence: 255, // Default confidence
                tags: request.metadata
                    .as_ref()
                    .and_then(|m| m.get("tags"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|s| s.as_str())
                        .map(String::from)
                        .collect())
                    .unwrap_or_default(),
                notes: request.metadata
                    .as_ref()
                    .and_then(|m| m.get("notes"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
            },
            objective_metadata: ObjectiveMetadata {
                created_at: now,
                source: request.source
                    .map(|s| MemorySource::information(s, "api".to_string()))
                    .unwrap_or_else(|| MemorySource::information("api".to_string(), "direct".to_string())),
            },
            associations: Vec::new(),
        };

        match state.memory_manager.append(&memory_fragment).await {
            Ok(()) => {
                info!("Successfully created memory fragment");
                // Return a mock response since we don't have the generated ID
                // In a real implementation, you'd modify append to return the created memory with ID
                let response = MemoryResponse {
                    id: 1, // Mock ID - would be returned from the database
                    content: memory_fragment.content,
                    metadata: Some(memory_fragment.subjective_metadata.notes.into()),
                    source: Some(memory_fragment.objective_metadata.source.identifier),
                    created_at: memory_fragment.objective_metadata.created_at,
                    updated_at: memory_fragment.objective_metadata.created_at, // Same as created_at for new memories
                };
                Ok(Json(ApiResponse::success(response)))
            }
            Err(e) => {
                error!("Failed to create memory fragment: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Search memory fragments
    #[instrument(skip(state))]
    pub async fn search_memory(
        State(state): State<AppState>,
        Query(request): Query<SearchMemoryRequest>,
    ) -> Result<Json<ApiResponse<SearchMemoryResponse>>, StatusCode> {
        info!("Searching memory fragments with keywords: {}", request.keywords);

        let query = request.into();

        match state.memory_manager.recall(&query).await {
            Ok(result) => {
                info!("Found {} memory fragments", result.memories.len());
                let response = SearchMemoryResponse::from(result);
                Ok(Json(ApiResponse::success(response)))
            }
            Err(e) => {
                error!("Failed to search memory fragments: {}", e);
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
                let response = MemoryResponse {
                    id: memory_fragment.id,
                    content: memory_fragment.content,
                    metadata: Some(memory_fragment.subjective_metadata.notes.into()),
                    source: Some(memory_fragment.objective_metadata.source.identifier),
                    created_at: memory_fragment.objective_metadata.created_at,
                    updated_at: memory_fragment.objective_metadata.created_at, // Same as created_at for now
                };
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
    ) -> Result<Json<ApiResponse<Value>>, StatusCode> {
        info!("Deleting memory fragment with ID: {}", id);

        match state.memory_manager.delete(id).await {
            Ok(()) => {
                info!("Successfully deleted memory fragment with ID: {}", id);
                Ok(Json(ApiResponse::success(serde_json::json!({"deleted": true}))))
            }
            Err(e) => {
                error!("Failed to delete memory fragment with ID {}: {}", id, e);
                Err(StatusCode::NOT_FOUND)
            }
        }
    }
}