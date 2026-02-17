use async_trait::async_trait;
use thiserror::Error;

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

    /// Calculate importance score for a memory based on various factors
    fn calculate_importance(&self, memory: &MemoryFragment) -> u8 {
        let mut score = 0;

        // Content length factor (longer content is more important)
        score += (memory.content.len().min(1000) / 100) as u8;

        // Confidence factor
        score += memory.subjective_metadata.confidence;

        // Tags factor (more tags = more important)
        score += memory.subjective_metadata.tags.len().min(5) as u8 * 2;

        score.min(100)
    }

    /// Get a specific memory by ID
    pub async fn get(&self, id: i64) -> Result<MemoryFragment, HybridError> {
        self.mysql_manager.get_one(id).await.map_err(HybridError::from)
    }

    /// Delete a memory by ID
    pub async fn delete(&self, id: i64) -> Result<(), HybridError> {
        self.mysql_manager.delete(&[id]).await.map_err(HybridError::from)
    }
}

#[async_trait]
impl Manager for HybridMemoryManager {
    type Error = HybridError;

    async fn append(&self, fragments: &mut Vec<MemoryFragment>) -> Result<Vec<i64>, HybridError> {
        if fragments.is_empty() {
            return Ok(vec![]);
        }

        // Calculate and update importance scores for all fragments
        for fragment in fragments.iter_mut() {
            fragment.subjective_metadata.importance = self.calculate_importance(fragment);
        }

        self.mysql_manager.save(fragments).await.map_err(HybridError::from)
    }

    async fn recall(&self, _query: &MemoryQuery) -> Result<MemoryQueryResult, HybridError> {
        // TODO: Implement recall functionality
        // Currently returns empty results, to be implemented later
        Ok(MemoryQueryResult { memories: vec![] })
    }
}
