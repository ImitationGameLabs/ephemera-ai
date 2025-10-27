use qdrant_client::Qdrant;
use thiserror::Error;

use crate::memory::types::MemoryFragment;

#[derive(Error, Debug)]
pub enum QdrantError {
    #[error("Qdrant client error: {0}")]
    Client(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Embedding generation error: {0}")]
    Embedding(String),
}

const QDRANT_COLLECTION_NAME: &str = "ephemera_memory";

pub struct QdrantMemoryManager {
    client: Qdrant,
    vector_dimensions: usize,
}

impl QdrantMemoryManager {
    pub fn new(client: Qdrant, vector_dimensions: usize) -> Self {
        Self {
            client,
            vector_dimensions,
        }
    }

    pub async fn ensure_collection(&self) -> Result<(), QdrantError> {
        if !self
            .client
            .collection_exists(QDRANT_COLLECTION_NAME)
            .await
            .map_err(|e| QdrantError::Client(e.to_string()))?
        {
            self.client
                .create_collection(
                    qdrant_client::qdrant::CreateCollectionBuilder::new(QDRANT_COLLECTION_NAME)
                        .vectors_config(qdrant_client::qdrant::VectorParamsBuilder::new(
                            self.vector_dimensions as u64,
                            qdrant_client::qdrant::Distance::Cosine,
                        )),
                )
                .await
                .map_err(|e| QdrantError::Client(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn save_embedding(
        &self,
        memory: &MemoryFragment,
        embedding: Vec<f32>,
    ) -> Result<(), QdrantError> {
        self.ensure_collection().await?;

        let point = qdrant_client::qdrant::PointStruct::new(
            memory.id as u64,
            embedding,
            [
                ("created_at", memory.objective_metadata.created_at.into()),
                (
                    "source",
                    format!("{:?}", memory.objective_metadata.source).into(),
                ),
                (
                    "importance",
                    (memory.subjective_metadata.importance as f32).into(),
                ),
                (
                    "confidence",
                    (memory.subjective_metadata.confidence as f32).into(),
                ),
                (
                    "tags",
                    serde_json::to_string(&memory.subjective_metadata.tags)?.into(),
                ),
            ],
        );

        self.client
            .upsert_points(qdrant_client::qdrant::UpsertPointsBuilder::new(
                QDRANT_COLLECTION_NAME,
                vec![point],
            ))
            .await
            .map_err(|e| QdrantError::Client(e.to_string()))?;

        Ok(())
    }

    pub async fn search_similar(
        &self,
        query_embedding: Vec<f32>,
        limit: u32,
        filter: Option<qdrant_client::qdrant::Filter>,
    ) -> Result<Vec<i64>, QdrantError> {
        self.ensure_collection().await?;

        let mut search_builder = qdrant_client::qdrant::SearchPointsBuilder::new(
            QDRANT_COLLECTION_NAME,
            query_embedding,
            limit.into(),
        )
        .with_payload(true);

        if let Some(filter) = filter {
            search_builder = search_builder.filter(filter);
        }

        let search_result = self
            .client
            .search_points(search_builder)
            .await
            .map_err(|e| QdrantError::Client(e.to_string()))?;

        let ids = search_result
            .result
            .iter()
            .filter_map(|point| point.id.as_ref())
            .filter_map(|id| match id.point_id_options.as_ref()? {
                qdrant_client::qdrant::point_id::PointIdOptions::Num(num) => Some(*num as i64),
                _ => None,
            })
            .collect();

        Ok(ids)
    }

    pub async fn delete_embedding(&self, id: i64) -> Result<(), QdrantError> {
        self.client
            .delete_points(
                qdrant_client::qdrant::DeletePointsBuilder::new(QDRANT_COLLECTION_NAME)
                    .points(vec![id as u64]),
            )
            .await
            .map_err(|e| QdrantError::Client(e.to_string()))?;

        Ok(())
    }
}
