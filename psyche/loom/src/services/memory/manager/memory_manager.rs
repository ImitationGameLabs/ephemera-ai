use sea_orm::*;
use thiserror::Error;
use time::OffsetDateTime;

use crate::memory::types::MemoryFragment;
use crate::services::memory::entity::{self, Entity as MemoryEntity};

/// Error type for memory operations
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    DbError(#[from] DbErr),

    #[error("Memory fragment not found: {0}")]
    NotFound(i64),
}

/// Memory manager for storing and retrieving memory fragments
pub struct MemoryManager {
    db: DatabaseConnection,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(db: DatabaseConnection, _machine_id: u16) -> Self {
        Self { db }
    }

    /// Append memory fragments to the store
    pub async fn append(&self, fragments: &mut Vec<MemoryFragment>) -> Result<Vec<i64>, MemoryError> {
        if fragments.is_empty() {
            return Ok(vec![]);
        }

        self.save(fragments).await
    }

    /// Save memory fragments, auto-generating IDs
    async fn save(&self, memories: &mut [MemoryFragment]) -> Result<Vec<i64>, MemoryError> {
        let mut ids = Vec::with_capacity(memories.len());

        for fragment in memories.iter_mut() {
            let active_model = entity::ActiveModel {
                id: NotSet,
                content: Set(fragment.content.clone()),
                timestamp: Set(fragment.timestamp),
                kind: Set(fragment.kind.as_tag().to_string()),
            };

            let inserted = active_model.insert(&self.db).await?;
            fragment.id = inserted.id;
            ids.push(inserted.id);
        }

        Ok(ids)
    }

    /// Get a single memory fragment by ID
    pub async fn get_one(&self, id: i64) -> Result<MemoryFragment, MemoryError> {
        let model = MemoryEntity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(MemoryError::NotFound(id))?;

        Ok(model.into())
    }

    /// Delete memory fragments by IDs
    pub async fn delete(&self, ids: &[i64]) -> Result<(), MemoryError> {
        for id in ids {
            MemoryEntity::delete_by_id(*id)
                .exec(&self.db)
                .await?;
        }

        Ok(())
    }

    /// Get recent memory fragments
    pub async fn get_recent(&self, limit: usize) -> Result<Vec<MemoryFragment>, MemoryError> {
        let models = MemoryEntity::find()
            .order_by_desc(entity::Column::Timestamp)
            .limit(limit as u64)
            .all(&self.db)
            .await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    /// Get memory fragments within a time range
    pub async fn get_range(
        &self,
        start: OffsetDateTime,
        end: OffsetDateTime,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<MemoryFragment>, MemoryError> {
        let mut query = MemoryEntity::find()
            .filter(entity::Column::Timestamp.gte(start))
            .filter(entity::Column::Timestamp.lte(end))
            .order_by_desc(entity::Column::Timestamp);

        if let Some(lim) = limit {
            query = query.limit(lim as u64);
        }

        if let Some(off) = offset {
            query = query.offset(off as u64);
        }

        let models = query.all(&self.db).await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }
}
