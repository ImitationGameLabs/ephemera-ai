use async_trait::async_trait;
use rig::embeddings::embedding::EmbeddingModelDyn;
use thiserror::Error;

use super::{
    Manager, MemoryQuery, MemoryQueryResult,
    mysql_manager::{MysqlError, MysqlMemoryManager},
    qdrant_manager::{QdrantError, QdrantMemoryManager},
};
use crate::memory::{
    types::MemoryFragment
};

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
    embedding_model: Box<dyn EmbeddingModelDyn>,
}

impl HybridMemoryManager {
    pub fn new(
        mysql_manager: MysqlMemoryManager,
        qdrant_manager: QdrantMemoryManager,
        embedding_model: Box<dyn EmbeddingModelDyn>,
    ) -> Self {
        Self {
            mysql_manager,
            qdrant_manager,
            embedding_model,
        }
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, HybridError> {
        let embedding = self.embedding_model.embed_text(text)
            .await
            .map_err(|e| HybridError::Embedding(format!("Failed to generate embedding: {e}")))?;

        Ok(embedding.vec.into_iter().map(|x| x as f32).collect())
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

    async fn build_qdrant_filter(
        &self,
        _query: &MemoryQuery,
    ) -> Result<Option<qdrant_client::qdrant::Filter>, HybridError> {
        // For now, skip complex filtering to avoid Qdrant API compatibility issues
        // TODO: Implement proper filtering when Qdrant API is more stable
        Ok(None)
    }

    /// Get a specific memory by ID
    pub async fn get(&self, id: i64) -> Result<MemoryFragment, HybridError> {
        self.mysql_manager.get_one(id).await.map_err(HybridError::from)
    }

    /// Delete a memory by ID (removes from both MySQL and Qdrant)
    pub async fn delete(&self, id: i64) -> Result<(), HybridError> {
        // Delete from MySQL first
        self.mysql_manager.delete(&[id]).await.map_err(HybridError::from)?;

        // Delete from Qdrant (if fails, log error but don't fail the operation)
        if let Err(e) = self.qdrant_manager.delete_embedding(id).await {
            eprintln!("Warning: Failed to delete embedding from Qdrant for memory {}: {}", id, e);
        }

        Ok(())
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

        // Save to MySQL first and get the generated IDs
        let generated_ids = self.mysql_manager.save(&fragments).await?;

        // Update fragments with their generated IDs for Qdrant storage
        for (fragment, id) in fragments.iter_mut().zip(generated_ids.iter()) {
            fragment.id = *id;
        }

        // Generate embeddings for all fragments and save to Qdrant
        let mut embeddings = Vec::new();
        for fragment in fragments.iter() {
            let embedding = self
                .generate_embedding(&fragment.content)
                .await
                .map_err(|e| HybridError::Embedding(e.to_string()))?;
            embeddings.push(embedding);
        }

        // Save all embeddings to Qdrant
        match self
            .qdrant_manager
            .save(&fragments, &embeddings)
            .await
        {
            Ok(_) => Ok(generated_ids),
            Err(_qdrant_error) => {
                // Qdrant failed, roll back MySQL records
                if let Err(mysql_error) = self.mysql_manager.delete(&generated_ids).await {
                    // If rollback also fails, we have a bigger problem
                    // Log this situation for manual intervention
                    eprintln!(
                        "CRITICAL: Failed to rollback MySQL records {:?} after Qdrant failure: {}",
                        generated_ids, mysql_error
                    );
                }
                Err(HybridError::TransactionRollback(generated_ids[0])) // Use first ID as representative
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
            self.mysql_manager.get(&similar_ids).await?
        };

        Ok(MemoryQueryResult { memories })
    }
}