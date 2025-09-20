mod hybrid_manager;
mod mysql_manager;
mod qdrant_manager;

pub use hybrid_manager::*;
pub use mysql_manager::*;
pub use qdrant_manager::*;

use serde::{Deserialize, Serialize};

use crate::MemoryFragment;

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

/// Result of a memory query operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResult {
    pub memories: Vec<MemoryFragment>,
}

use async_trait::async_trait;

/// Trait defining the interface for memory management operations
#[async_trait]
pub trait Manager {
    type Error;

    async fn append(&mut self, memory: &MemoryFragment) -> Result<(), Self::Error>;
    async fn recall(&self, query: &MemoryQuery) -> Result<MemoryQueryResult, Self::Error>;
}
