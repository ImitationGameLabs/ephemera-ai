use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, NotSet, TransactionTrait};
use thiserror::Error;

use crate::memory::types::MemoryFragment;
use crate::services::memory::entity::MemoryEntity;
use crate::services::memory::entity::memory;

#[derive(Error, Debug)]
pub enum MysqlError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sea_orm::DbErr),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Memory not found with id: {0}")]
    NotFound(i64),
}

pub struct MysqlMemoryManager {
    conn: DatabaseConnection,
}

impl MysqlMemoryManager {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    pub async fn save(&self, memories: &[MemoryFragment]) -> Result<Vec<i64>, MysqlError> {
        let mut inserted_ids = Vec::new();

        // Use transaction for batch insert
        let txn = self.conn.begin().await?;

        for memory in memories {
            let model: memory::Model = memory.clone().into();
            let active_model = memory::ActiveModel {
                // Let database auto-generate ID
                id: NotSet,
                content: Set(model.content),
                created_at: Set(model.created_at),
                updated_at: Set(model.updated_at),
                source: Set(model.source),
                importance: Set(model.importance),
                confidence: Set(model.confidence),
                tags: Set(model.tags),
                notes: Set(model.notes),
                associations: Set(model.associations),
            };

            let inserted_model = active_model.insert(&txn).await?;
            inserted_ids.push(inserted_model.id);
        }

        // Commit transaction
        txn.commit().await?;

        Ok(inserted_ids)
    }

    pub async fn get(&self, ids: &[i64]) -> Result<Vec<MemoryFragment>, MysqlError> {
        let models = MemoryEntity::find()
            .filter(memory::Column::Id.is_in(ids.iter().copied()))
            .all(&self.conn)
            .await?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    pub async fn get_one(&self, id: i64) -> Result<MemoryFragment, MysqlError> {
        let model = MemoryEntity::find_by_id(id)
            .one(&self.conn)
            .await?
            .ok_or(MysqlError::NotFound(id))?;

        Ok(model.into())
    }

    pub async fn delete(&self, ids: &[i64]) -> Result<(), MysqlError> {
        let result = MemoryEntity::delete_many()
            .filter(memory::Column::Id.is_in(ids.iter().copied()))
            .exec(&self.conn)
            .await?;

        if result.rows_affected == 0 {
            return Err(MysqlError::NotFound(0)); // Use 0 as indicator for batch not found
        }

        Ok(())
    }

}
