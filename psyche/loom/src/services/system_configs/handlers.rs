use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use tracing::{error, info, instrument};

use crate::system_configs::models::{CreateSystemConfigRequest, SystemConfigQuery, SystemConfigResponse};
use crate::services::system_configs::manager::SystemConfigError;
use crate::services::system_configs::AppState;

/// HTTP handler for system configs operations
pub struct SystemConfigHandler;

/// Query parameters for system configs API
#[derive(Debug, Deserialize)]
pub struct SystemConfigQueryParams {
    pub memory_fragment_id: Option<i64>,
    pub content_hash: Option<String>,
    pub start_time: Option<String>, // ISO 8601 format
    pub end_time: Option<String>,   // ISO 8601 format
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

impl SystemConfigHandler {
    /// Create a system config record
    #[instrument(skip(state))]
    pub async fn create_system_config(
        State(state): State<AppState>,
        Json(request): Json<CreateSystemConfigRequest>,
    ) -> Result<Json<SystemConfigResponse>, StatusCode> {
        info!("Creating system config record");

        match state.system_config_manager.create(request).await {
            Ok(record) => {
                info!("Successfully created system config record with id: {}", record.id);
                Ok(Json(SystemConfigResponse::single(record)))
            }
            Err(SystemConfigError::AlreadyExists(hash)) => {
                info!("System config record already exists with content hash: {}", hash);
                Err(StatusCode::CONFLICT)
            }
            Err(e) => {
                error!("Failed to create system config record: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Query system config records
    #[instrument(skip(state))]
    pub async fn query_system_configs(
        State(state): State<AppState>,
        Query(params): Query<SystemConfigQueryParams>,
    ) -> Result<Json<SystemConfigResponse>, StatusCode> {
        info!("Querying system config records");

        // Parse query parameters
        let query = SystemConfigQuery {
            memory_fragment_id: params.memory_fragment_id,
            content_hash: params.content_hash,
            start_time: params.start_time
                .and_then(|s| time::OffsetDateTime::parse(&s, &time::format_description::well_known::Iso8601::DEFAULT).ok()),
            end_time: params.end_time
                .and_then(|s| time::OffsetDateTime::parse(&s, &time::format_description::well_known::Iso8601::DEFAULT).ok()),
            limit: params.limit,
            offset: params.offset,
        };

        match state.system_config_manager.query(query).await {
            Ok((records, total)) => {
                info!("Found {} system config records (total: {})", records.len(), total);
                Ok(Json(SystemConfigResponse::multiple(records)))
            }
            Err(e) => {
                error!("Failed to query system config records: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    /// Get system config by ID
    #[instrument(skip(state))]
    pub async fn get_system_config(
        State(state): State<AppState>,
        Path(id): Path<i64>,
    ) -> Result<Json<SystemConfigResponse>, StatusCode> {
        info!("Getting system config record with id: {}", id);

        match state.system_config_manager.get_by_id(id).await {
            Ok(record) => {
                info!("Successfully retrieved system config record with id: {}", id);
                Ok(Json(SystemConfigResponse::single(record)))
            }
            Err(SystemConfigError::NotFound(_)) => {
                info!("System config record not found with id: {}", id);
                Err(StatusCode::NOT_FOUND)
            }
            Err(e) => {
                error!("Failed to get system config record: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
