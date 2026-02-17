use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use crate::memory::types::MemoryFragment;

/// Represents a time range for memory queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

/// Query parameters for memory retrieval operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub keywords: String,
    pub time_range: Option<TimeRange>,
}

/// Unified response model for memory operations (single or multiple fragments)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryResponse {
    pub fragments: Vec<MemoryFragment>,
    pub total: usize,
}

impl MemoryResponse {
    /// Create a response with a single memory fragment
    pub fn single(fragment: MemoryFragment) -> Self {
        Self {
            fragments: vec![fragment],
            total: 1,
        }
    }

    /// Create a response with multiple memory fragments
    pub fn multiple(fragments: Vec<MemoryFragment>) -> Self {
        let total = fragments.len();
        Self {
            fragments,
            total,
        }
    }

    /// Get the first fragment (convenience method for single fragment responses)
    pub fn first(&self) -> Option<&MemoryFragment> {
        self.fragments.first()
    }

    /// Check if response contains any fragments
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// Get the number of fragments
    pub fn len(&self) -> usize {
        self.fragments.len()
    }
}


/// Request model for creating memory fragments (supports batch operations)
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMemoryRequest {
    pub fragments: Vec<MemoryFragment>,
}

impl CreateMemoryRequest {
    /// Create a request with a single memory fragment (backward compatibility)
    pub fn single(fragment: MemoryFragment) -> Self {
        Self {
            fragments: vec![fragment],
        }
    }

    /// Create a request with multiple memory fragments
    pub fn multiple(fragments: Vec<MemoryFragment>) -> Self {
        Self { fragments }
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

/// Legacy type alias for backward compatibility
pub type SearchMemoryResponse = MemoryResponse;

/// Legacy type for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResult {
    pub memories: Vec<MemoryFragment>,
}

impl From<MemoryQueryResult> for MemoryResponse {
    fn from(result: MemoryQueryResult) -> Self {
        MemoryResponse::multiple(result.memories)
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