use async_trait::async_trait;
use thiserror::Error;

use super::{
    mysql_manager::{MysqlError, MysqlMemoryManager},
    qdrant_manager::{QdrantError, QdrantMemoryManager},
};
use crate::{Manager, MemoryFragment, MemoryQuery, MemoryQueryResult};

/// Filter criteria for memory queries
#[derive(Debug)]
pub struct MemoryQueryFilter {
    pub min_importance: Option<u8>,
    pub min_confidence: Option<u8>,
    pub tags: Option<Vec<String>>,
    pub time_range: Option<crate::manager::TimeRange>,
}

#[derive(Error, Debug)]
pub enum HybridError {
    #[error("MySQL error: {0}")]
    Mysql(#[from] MysqlError),

    #[error("Qdrant error: {0}")]
    Qdrant(#[from] QdrantError),

    #[error("Embedding service error: {0}")]
    Embedding(String),

    #[error(
        "Transaction failed: MySQL succeeded but Qdrant failed, MySQL record with id {0} was rolled back"
    )]
    TransactionRollback(i64),
}

pub struct HybridMemoryManager {
    mysql_manager: MysqlMemoryManager,
    qdrant_manager: QdrantMemoryManager,
}

impl HybridMemoryManager {
    pub fn new(mysql_manager: MysqlMemoryManager, qdrant_manager: QdrantMemoryManager) -> Self {
        Self {
            mysql_manager,
            qdrant_manager,
        }
    }

    async fn generate_embedding(&self, _text: &str) -> Result<Vec<f32>, HybridError> {
        // TODO: Implement actual embedding generation
        // For now, return a dummy embedding
        Ok(vec![0.1; 384])
    }
}

#[async_trait]
impl Manager for HybridMemoryManager {
    type Error = HybridError;

    async fn append(&mut self, memory: &MemoryFragment) -> Result<(), HybridError> {
        // Save to MySQL first
        self.mysql_manager.save(memory).await?;

        // Generate embedding and save to Qdrant
        let embedding = self
            .generate_embedding(&memory.content)
            .await
            .map_err(|e| HybridError::Embedding(e.to_string()))?;

        match self.qdrant_manager.save_embedding(memory, embedding).await {
            Ok(_) => Ok(()),
            Err(_qdrant_error) => {
                // Qdrant failed, roll back MySQL transaction
                if let Err(mysql_error) = self.mysql_manager.delete(memory.id).await {
                    // If rollback also fails, we have a bigger problem
                    // Log this situation for manual intervention
                    eprintln!(
                        "CRITICAL: Failed to rollback MySQL record {} after Qdrant failure: {}",
                        memory.id, mysql_error
                    );
                }
                Err(HybridError::TransactionRollback(memory.id))
            }
        }
    }

    async fn recall(&self, query: &MemoryQuery) -> Result<MemoryQueryResult, HybridError> {
        // Generate embedding for query
        let query_embedding = self
            .generate_embedding(&query.keywords)
            .await
            .map_err(|e| HybridError::Embedding(e.to_string()))?;

        // Build Qdrant filter from query parameters
        let filter = self.build_qdrant_filter(query).await?;

        // Search similar memories in Qdrant
        let similar_ids = self
            .qdrant_manager
            .search_similar(
                query_embedding,
                10, // Default limit
                filter,
            )
            .await?;

        // Fetch full memory data from MySQL
        let memories = if similar_ids.is_empty() {
            Vec::new()
        } else {
            self.mysql_manager.get_batch(&similar_ids).await?
        };

        Ok(MemoryQueryResult { memories })
    }
}

impl HybridMemoryManager {
    async fn build_qdrant_filter(
        &self,
        _query: &MemoryQuery,
    ) -> Result<Option<qdrant_client::qdrant::Filter>, HybridError> {
        // For now, skip complex filtering to avoid Qdrant API compatibility issues
        // TODO: Implement proper filtering when Qdrant API is more stable
        Ok(None)
    }
}
