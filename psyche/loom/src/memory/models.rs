use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use crate::memory::{
    types::MemoryFragment,
    manager::{MemoryQuery, MemoryQueryResult},
};
use crate::memory::manager::TimeRange;

/// Request model for creating a new memory fragment
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMemoryRequest {
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub source: Option<String>,
}

/// Response model for memory operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResponse {
    pub id: i64,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub source: Option<String>,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_at: OffsetDateTime,
}

impl From<MemoryFragment> for MemoryResponse {
    fn from(fragment: MemoryFragment) -> Self {
        Self {
            id: fragment.id,
            content: fragment.content,
            metadata: Some(fragment.subjective_metadata.notes.into()),
            source: Some(fragment.objective_metadata.source.identifier),
            created_at: OffsetDateTime::from_unix_timestamp(fragment.objective_metadata.created_at)
                .unwrap_or_else(|_| OffsetDateTime::now_utc()),
            updated_at: OffsetDateTime::from_unix_timestamp(fragment.objective_metadata.created_at)
                .unwrap_or_else(|_| OffsetDateTime::now_utc()), // Same as created_at for now
        }
    }
}

/// Request model for memory search
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchMemoryRequest {
    pub keywords: String,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
}

impl From<SearchMemoryRequest> for MemoryQuery {
    fn from(request: SearchMemoryRequest) -> Self {
        Self {
            keywords: request.keywords,
            time_range: if let (Some(start), Some(end)) = (request.start_time, request.end_time) {
                Some(TimeRange { start, end })
            } else {
                None
            },
        }
    }
}

/// Response model for memory search results
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchMemoryResponse {
    pub memories: Vec<MemoryResponse>,
    pub total: usize,
}

impl From<MemoryQueryResult> for SearchMemoryResponse {
    fn from(result: MemoryQueryResult) -> Self {
        let total = result.memories.len();
        Self {
            memories: result.memories.into_iter().map(MemoryResponse::from).collect(),
            total,
        }
    }
}

/// Standard API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}