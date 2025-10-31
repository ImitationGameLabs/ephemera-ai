mod hybrid_manager;
mod mysql_manager;
mod qdrant_manager;

pub use hybrid_manager::*;
pub use mysql_manager::*;
pub use qdrant_manager::*;

use async_trait::async_trait;

use crate::memory::types::MemoryFragment;
use crate::memory::models::{MemoryQuery, MemoryQueryResult};

/// Trait defining the interface for memory management operations
#[async_trait]
pub trait Manager {
    type Error;

    async fn append(&self, fragments: &mut Vec<MemoryFragment>) -> Result<Vec<i64>, Self::Error>;
    async fn recall(&self, query: &MemoryQuery) -> Result<MemoryQueryResult, Self::Error>;
}
