use async_trait::async_trait;
use thiserror::Error;
use time::OffsetDateTime;

use super::{
    Manager, MemoryQuery, MemoryQueryResult,
    mysql_manager::{MysqlError, MysqlMemoryManager},
};
use crate::memory::types::MemoryFragment;

#[derive(Error, Debug)]
pub enum HybridError {
    #[error("MySQL error: {0}")]
    Mysql(#[from] MysqlError),
}

pub struct HybridMemoryManager {
    mysql_manager: MysqlMemoryManager,
}

impl HybridMemoryManager {
    pub fn new(mysql_manager: MysqlMemoryManager) -> Self {
        Self { mysql_manager }
    }

    /// Get a specific memory by ID
    pub async fn get(&self, id: i64) -> Result<MemoryFragment, HybridError> {
        self.mysql_manager
            .get_one(id)
            .await
            .map_err(HybridError::from)
    }

    /// Delete a memory by ID
    pub async fn delete(&self, id: i64) -> Result<(), HybridError> {
        self.mysql_manager
            .delete(&[id])
            .await
            .map_err(HybridError::from)
    }

    /// Get the most recent memories
    pub async fn get_recent(&self, limit: usize) -> Result<Vec<MemoryFragment>, HybridError> {
        self.mysql_manager
            .get_recent(limit)
            .await
            .map_err(HybridError::from)
    }

    /// Get memories within a time range
    pub async fn get_range(
        &self,
        start: OffsetDateTime,
        end: OffsetDateTime,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<MemoryFragment>, HybridError> {
        self.mysql_manager
            .get_range(start, end, limit, offset)
            .await
            .map_err(HybridError::from)
    }
}

#[async_trait]
impl Manager for HybridMemoryManager {
    type Error = HybridError;

    async fn append(&self, fragments: &mut Vec<MemoryFragment>) -> Result<Vec<i64>, HybridError> {
        if fragments.is_empty() {
            return Ok(vec![]);
        }

        self.mysql_manager
            .save(fragments)
            .await
            .map_err(HybridError::from)
    }

    async fn recall(&self, _query: &MemoryQuery) -> Result<MemoryQueryResult, HybridError> {
        // TODO: Implement recall functionality
        // Currently returns empty results, to be implemented later
        Ok(MemoryQueryResult { memories: vec![] })
    }
}
