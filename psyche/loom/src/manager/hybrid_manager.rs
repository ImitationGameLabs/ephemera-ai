use async_trait::async_trait;
use rig::embeddings::embedding::EmbeddingModelDyn;
use thiserror::Error;

use super::{
    mysql_manager::{MysqlError, MysqlMemoryManager},
    qdrant_manager::{QdrantError, QdrantMemoryManager},
};
use crate::{Manager, MemoryFragment, MemoryQuery, MemoryQueryResult, TimeRange};

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

    #[error("Reflection error: {0}")]
    Reflection(String),

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

    /// Analyze recent memories (last 24 hours) for immediate patterns
    async fn analyze_recent_memories(&self) -> Result<Vec<String>, HybridError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let one_day_ago = now - 24 * 60 * 60;

        // Use a broad query to find recent memories
        let query = MemoryQuery {
            keywords: "recent today now current".to_string(),
            time_range: Some(TimeRange {
                start: one_day_ago,
                end: now,
            }),
        };

        let result = self.recall(&query).await?;

        if result.memories.len() > 5 {
            Ok(vec![format!("Found {} recent memories from the last 24 hours", result.memories.len())])
        } else {
            Ok(Vec::new())
        }
    }

    /// Analyze important memories for significant insights
    async fn analyze_important_memories(&self) -> Result<Vec<String>, HybridError> {
        let important_memories = self.get_important_memories(80).await?;

        if !important_memories.is_empty() {
            Ok(vec![format!("Found {} high-importance memories (importance >= 80)", important_memories.len())])
        } else {
            Ok(Vec::new())
        }
    }

    /// Analyze thematic patterns through semantic search
    async fn analyze_thematic_patterns(&self) -> Result<Vec<String>, HybridError> {
        // Use common themes to detect patterns
        let themes = ["learning", "problem", "solution", "idea", "goal"];
        let mut thematic_insights = Vec::new();

        for theme in themes {
            let query = MemoryQuery {
                keywords: theme.to_string(),
                time_range: None,
            };

            let result = self.recall(&query).await?;
            if result.memories.len() >= 3 {
                thematic_insights.push(format!("Theme '{}' appears in {} memories", theme, result.memories.len()));
            }
        }

        Ok(thematic_insights)
    }

        async fn build_qdrant_filter(
        &self,
        _query: &MemoryQuery,
    ) -> Result<Option<qdrant_client::qdrant::Filter>, HybridError> {
        // For now, skip complex filtering to avoid Qdrant API compatibility issues
        // TODO: Implement proper filtering when Qdrant API is more stable
        Ok(None)
    }

    /// Perform reflection on stored memories to identify patterns and insights
    pub async fn reflect(&self) -> Result<Vec<String>, HybridError> {
        let mut insights = Vec::new();

        // Analyze recent memories for immediate patterns
        insights.extend(self.analyze_recent_memories().await?);

        // Analyze important memories for significant insights
        insights.extend(self.analyze_important_memories().await?);

        // Analyze thematic patterns through semantic search
        insights.extend(self.analyze_thematic_patterns().await?);

        if insights.is_empty() {
            Ok(vec![
                "No significant patterns detected yet. Keep building memories.".to_string(),
            ])
        } else {
            Ok(insights)
        }
    }

    /// Get memories with high importance for review using targeted queries
    pub async fn get_important_memories(
        &self,
        min_importance: u8,
    ) -> Result<Vec<MemoryFragment>, HybridError> {
        // Use a broad query to find important memories
        // In a real implementation, this would use proper database filtering
        let query = MemoryQuery {
            keywords: "important significant crucial critical".to_string(),
            time_range: None,
        };

        let result = self.recall(&query).await?;

        // Filter locally for importance (temporary until proper DB filtering)
        let important_memories = result.memories
            .into_iter()
            .filter(|m| m.subjective_metadata.importance >= min_importance)
            .collect();

        Ok(important_memories)
    }

    /// Prune low-importance memories to maintain database efficiency
    /// This is a placeholder - proper pruning would require database-level operations
    pub async fn prune_memories(&self, _max_memories: usize) -> Result<usize, HybridError> {
        // Pruning should be implemented at the database level with proper queries
        // rather than loading all memories into memory
        // For now, return 0 (no pruning) until proper implementation
        Ok(0)
    }
}

#[async_trait]
impl Manager for HybridMemoryManager {
    type Error = HybridError;

    async fn append(&self, memory: &MemoryFragment) -> Result<(), HybridError> {
        // Calculate and update importance score before saving
        let mut memory_with_importance = memory.clone();
        memory_with_importance.subjective_metadata.importance = self.calculate_importance(memory);

        // Save to MySQL first and get the generated ID
        let generated_id = self.mysql_manager.save(&memory_with_importance).await?;

        // Update the memory object with the generated ID for Qdrant storage
        memory_with_importance.id = generated_id;

        // Generate embedding and save to Qdrant
        let embedding = self
            .generate_embedding(&memory_with_importance.content)
            .await
            .map_err(|e| HybridError::Embedding(e.to_string()))?;

        match self
            .qdrant_manager
            .save_embedding(&memory_with_importance, embedding)
            .await
        {
            Ok(_) => Ok(()),
            Err(_qdrant_error) => {
                // Qdrant failed, roll back MySQL transaction
                if let Err(mysql_error) = self.mysql_manager.delete(memory_with_importance.id).await
                {
                    // If rollback also fails, we have a bigger problem
                    // Log this situation for manual intervention
                    eprintln!(
                        "CRITICAL: Failed to rollback MySQL record {} after Qdrant failure: {}",
                        memory_with_importance.id, mysql_error
                    );
                }
                Err(HybridError::TransactionRollback(memory_with_importance.id))
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