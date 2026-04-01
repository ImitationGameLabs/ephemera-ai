use crate::types::MemoryFragment;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Represents a time range for memory queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

/// Request model for getting recent memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentMemoryRequest {
    /// Maximum number of memories to return (default: 10)
    pub limit: usize,
}

/// Request model for querying memories within a time range (timeline view)
/// Uses ISO 8601 format for timestamps (e.g., "2024-01-15T10:30:00Z" or "2024-01-15T10:30:00+08:00")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMemoryRequest {
    /// Start time in ISO 8601 format
    pub from: String,
    /// End time in ISO 8601 format
    pub to: String,
    /// Maximum number of memories to return
    pub limit: Option<usize>,
    /// Number of memories to skip (for pagination)
    pub offset: Option<usize>,
}

impl TimelineMemoryRequest {
    /// Parse ISO 8601 strings into OffsetDateTime
    pub fn parse(&self) -> Result<ParsedTimeRange, TimeParseError> {
        use time::format_description::well_known::Iso8601;

        let start = OffsetDateTime::parse(&self.from, &Iso8601::PARSING)
            .map_err(|e| TimeParseError::InvalidFromTime(e.to_string()))?;
        let end = OffsetDateTime::parse(&self.to, &Iso8601::PARSING)
            .map_err(|e| TimeParseError::InvalidToTime(e.to_string()))?;

        Ok(ParsedTimeRange { start, end })
    }
}

/// Parsed time range with OffsetDateTime values
#[derive(Debug, Clone)]
pub struct ParsedTimeRange {
    pub start: OffsetDateTime,
    pub end: OffsetDateTime,
}

/// Error type for time parsing
#[derive(Debug, thiserror::Error)]
pub enum TimeParseError {
    #[error("Invalid from time: {0}")]
    InvalidFromTime(String),
    #[error("Invalid to time: {0}")]
    InvalidToTime(String),
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
        Self { fragments: vec![fragment], total: 1 }
    }

    /// Create a response with multiple memory fragments
    pub fn multiple(fragments: Vec<MemoryFragment>) -> Self {
        let total = fragments.len();
        Self { fragments, total }
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
        Self { fragments: vec![fragment] }
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
        Self { success: true, data: Some(data), error: None }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self { success: false, data: None, error: Some(message.into()) }
    }
}

// ============================================================================
// Pinned Memory Types
// ============================================================================

/// A pinned memory item that stays at the top of context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedMemory {
    /// The memory fragment that is pinned
    pub fragment: MemoryFragment,
    /// Reason for pinning this memory
    pub reason: Option<String>,
    /// When this memory was pinned
    #[serde(with = "time::serde::iso8601")]
    pub pinned_at: OffsetDateTime,
}

impl PinnedMemory {
    /// Create a new pinned memory
    pub fn new(fragment: MemoryFragment, reason: Option<String>) -> Self {
        Self { fragment, reason, pinned_at: OffsetDateTime::now_utc() }
    }
}

/// Request model for pinning a memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinMemoryRequest {
    /// ID of the memory to pin
    pub memory_id: i64,
    /// Reason for pinning
    pub reason: Option<String>,
}

/// Response model for listing pinned memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedMemoriesResponse {
    pub items: Vec<PinnedMemory>,
}

impl PinnedMemoriesResponse {
    pub fn new(items: Vec<PinnedMemory>) -> Self {
        Self { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}
