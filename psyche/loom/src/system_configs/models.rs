use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

// Forward declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigRecord {
    pub id: i64,
    pub content: String,
    pub content_hash: String,
    pub memory_fragment_id: Option<i64>,
    pub created_at: time::OffsetDateTime,
}

/// Request model for creating a system config record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSystemConfigRequest {
    pub content: String,
    pub memory_fragment_id: Option<i64>,
}

/// Query parameters for system config retrieval operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigQuery {
    pub memory_fragment_id: Option<i64>,
    pub content_hash: Option<String>,
    pub start_time: Option<OffsetDateTime>,
    pub end_time: Option<OffsetDateTime>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Response model for system config operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfigResponse {
    pub configs: Vec<SystemConfigRecord>,
    pub total: usize,
}

impl SystemConfigResponse {
    /// Create a response with a single system config record
    pub fn single(config: SystemConfigRecord) -> Self {
        Self {
            configs: vec![config],
            total: 1,
        }
    }

    /// Create a response with multiple system config records
    pub fn multiple(configs: Vec<SystemConfigRecord>) -> Self {
        let total = configs.len();
        Self {
            configs,
            total,
        }
    }
}
