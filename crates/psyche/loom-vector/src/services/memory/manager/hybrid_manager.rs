use async_trait::async_trait;
use rig::embeddings::embedding::EmbeddingModelDyn;
use thiserror::Error;

use super::{
    Manager, MemoryQuery, MemoryQueryResult,
    qdrant_manager::{QdrantError, QdrantMemoryManager},
};
use crate::memory::types::MemoryFragment;

#[derive(Error, Debug)]
pub enum VectorSearchError {
    #[error("Qdrant error: {0}")]
    Qdrant(#[from] QdrantError),

    #[error("Embedding service error: {0}")]
    Embedding(String),
}

/// Vector search manager for embedding-based memory search
pub struct VectorSearchManager {
    qdrant_manager: QdrantMemoryManager,
    embedding_model: Box<dyn EmbeddingModelDyn>,
}

impl VectorSearchManager {
    pub fn new(
        qdrant_manager: QdrantMemoryManager,
        embedding_model: Box<dyn EmbeddingModelDyn>,
    ) -> Self {
        Self {
            qdrant_manager,
            embedding_model,
        }
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, VectorSearchError> {
        let embedding = self.embedding_model.embed_text(text).await.map_err(|e| {
            VectorSearchError::Embedding(format!("Failed to generate embedding: {e}"))
        })?;

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
    ) -> Result<Option<qdrant_client::qdrant::Filter>, VectorSearchError> {
        // For now, skip complex filtering to avoid Qdrant API compatibility issues
        // TODO: Implement proper filtering when Qdrant API is more stable
        Ok(None)
    }

    /// Delete a memory embedding by ID
    pub async fn delete(&self, id: i64) -> Result<(), VectorSearchError> {
        self.qdrant_manager
            .delete_embedding(id)
            .await
            .map_err(VectorSearchError::from)
    }
}

#[async_trait]
impl Manager for VectorSearchManager {
    type Error = VectorSearchError;

    async fn append(
        &self,
        fragments: &mut Vec<MemoryFragment>,
    ) -> Result<Vec<i64>, VectorSearchError> {
        if fragments.is_empty() {
            return Ok(vec![]);
        }

        // Calculate and update importance scores for all fragments
        for fragment in fragments.iter_mut() {
            fragment.subjective_metadata.importance = self.calculate_importance(fragment);
        }

        // Generate embeddings for all fragments and save to Qdrant
        let mut embeddings = Vec::new();
        for fragment in fragments.iter() {
            let embedding = self
                .generate_embedding(&fragment.content)
                .await
                .map_err(|e| VectorSearchError::Embedding(e.to_string()))?;
            embeddings.push(embedding);
        }

        // Save all embeddings to Qdrant
        self.qdrant_manager
            .save(fragments, &embeddings)
            .await
            .map_err(VectorSearchError::from)?;

        // Return the IDs from fragments
        Ok(fragments.iter().map(|f| f.id).collect())
    }

    async fn recall(&self, query: &MemoryQuery) -> Result<MemoryQueryResult, VectorSearchError> {
        // Generate embedding for query
        let query_embedding = self
            .generate_embedding(&query.keywords)
            .await
            .map_err(|e| VectorSearchError::Embedding(e.to_string()))?;

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

        // TODO: Implement fetching full memory data from an external source
        // For now, return empty memories since we don't have MySQL access
        let memories = Vec::new();

        Ok(MemoryQueryResult { memories })
    }
}
