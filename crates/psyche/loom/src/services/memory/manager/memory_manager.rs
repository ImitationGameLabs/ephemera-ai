use sea_orm::*;

/// Check if the error is a unique constraint violation (race condition)
fn is_unique_constraint_violation(err: &DbErr) -> bool {
    // Check error string for MySQL unique constraint violation patterns
    let err_str = err.to_string();
    err_str.contains("1062") || err_str.contains("Duplicate entry")
}
use thiserror::Error;
use time::OffsetDateTime;

use crate::memory::models::PinnedMemory;
use crate::memory::types::MemoryFragment;
use crate::services::memory::entity::memory::{ActiveModel, Column, Entity as MemoryEntity};
use crate::services::memory::entity::pinned::{
    ActiveModel as PinnedActiveModel, Column as PinnedColumn, Entity as PinnedEntity,
};

/// Error type for memory operations
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    DbError(#[from] DbErr),

    #[error("Memory fragment not found: {0}")]
    NotFound(i64),

    #[error("Memory is pinned: {0}")]
    MemoryPinned(i64),

    #[error("Memory already pinned: {0}")]
    AlreadyPinned(i64),
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
    pub async fn append(&self, fragments: &mut [MemoryFragment]) -> Result<Vec<i64>, MemoryError> {
        if fragments.is_empty() {
            return Ok(vec![]);
        }

        self.save(fragments).await
    }

    /// Save memory fragments, auto-generating IDs
    async fn save(&self, memories: &mut [MemoryFragment]) -> Result<Vec<i64>, MemoryError> {
        let mut ids = Vec::with_capacity(memories.len());

        for fragment in memories.iter_mut() {
            let active_model = ActiveModel {
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
    /// Returns error if any of the memories are pinned
    pub async fn delete(&self, ids: &[i64]) -> Result<(), MemoryError> {
        for id in ids {
            // Check if memory is pinned
            let is_pinned = PinnedEntity::find_by_id(*id).one(&self.db).await?.is_some();

            if is_pinned {
                return Err(MemoryError::MemoryPinned(*id));
            }

            MemoryEntity::delete_by_id(*id).exec(&self.db).await?;
        }

        Ok(())
    }

    /// Get recent memory fragments
    pub async fn get_recent(&self, limit: usize) -> Result<Vec<MemoryFragment>, MemoryError> {
        let models = MemoryEntity::find()
            .order_by_desc(Column::Timestamp)
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
            .filter(Column::Timestamp.gte(start))
            .filter(Column::Timestamp.lte(end))
            .order_by_desc(Column::Timestamp);

        if let Some(lim) = limit {
            query = query.limit(lim as u64);
        }

        if let Some(off) = offset {
            query = query.offset(off as u64);
        }

        let models = query.all(&self.db).await?;

        Ok(models.into_iter().map(|m| m.into()).collect())
    }

    /// Pin a memory by ID
    pub async fn pin(
        &self,
        memory_id: i64,
        reason: Option<String>,
    ) -> Result<PinnedMemory, MemoryError> {
        // Get the memory fragment first (validates existence)
        let fragment = self.get_one(memory_id).await?;

        // Create pinned record
        let now = OffsetDateTime::now_utc();
        let active_model = PinnedActiveModel {
            memory_id: Set(memory_id),
            reason: Set(reason.clone()),
            pinned_at: Set(now),
        };

        // Insert and handle race condition: if another request inserted first,
        // we'll get a unique constraint violation (MySQL error 1062)
        match active_model.insert(&self.db).await {
            Ok(_) => Ok(PinnedMemory { fragment, reason, pinned_at: now }),
            Err(ref e) if is_unique_constraint_violation(e) => {
                Err(MemoryError::AlreadyPinned(memory_id))
            }
            Err(e) => Err(MemoryError::DbError(e)),
        }
    }

    /// Unpin a memory by ID
    pub async fn unpin(&self, memory_id: i64) -> Result<(), MemoryError> {
        let result = PinnedEntity::delete_by_id(memory_id).exec(&self.db).await?;

        if result.rows_affected == 0 {
            return Err(MemoryError::NotFound(memory_id));
        }

        Ok(())
    }

    /// Get all pinned memories
    pub async fn get_pinned(&self) -> Result<Vec<PinnedMemory>, MemoryError> {
        // Single JOIN query using find_also_related
        let results = PinnedEntity::find()
            .order_by_asc(PinnedColumn::PinnedAt)
            .find_also_related(MemoryEntity)
            .all(&self.db)
            .await?;

        let pinned_memories = results
            .into_iter()
            .filter_map(|(pinned, memory_opt)| {
                memory_opt.map(|memory_model| PinnedMemory {
                    fragment: memory_model.into(),
                    reason: pinned.reason,
                    pinned_at: pinned.pinned_at,
                })
            })
            .collect();

        Ok(pinned_memories)
    }

    /// Check if a memory is pinned
    #[allow(dead_code)]
    pub async fn is_pinned(&self, memory_id: i64) -> Result<bool, MemoryError> {
        let is_pinned = PinnedEntity::find_by_id(memory_id)
            .one(&self.db)
            .await?
            .is_some();

        Ok(is_pinned)
    }
}
