use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, NotSet};
use thiserror::Error;

use crate::MemoryFragment;
use crate::entity::MemoryEntity;
use crate::entity::memory;

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

    pub async fn save(&self, memory: &MemoryFragment) -> Result<i64, MysqlError> {
        let model: memory::Model = memory.clone().into();
        let active_model = memory::ActiveModel {
            // Let database auto-generate ID
            id: NotSet,
            content: Set(model.content),
            created_at: Set(model.created_at),
            source: Set(model.source),
            importance: Set(model.importance),
            confidence: Set(model.confidence),
            tags: Set(model.tags),
            notes: Set(model.notes),
            associations: Set(model.associations),
        };

        let inserted_model = active_model.insert(&self.conn).await?;
        Ok(inserted_model.id)
    }

    pub async fn get(&self, id: i64) -> Result<MemoryFragment, MysqlError> {
        let model = MemoryEntity::find_by_id(id)
            .one(&self.conn)
            .await?
            .ok_or(MysqlError::NotFound(id))?;

        Ok(model.into())
    }

    pub async fn get_batch(&self, ids: &[i64]) -> Result<Vec<MemoryFragment>, MysqlError> {
        let models = MemoryEntity::find()
            .filter(memory::Column::Id.is_in(ids.iter().copied()))
            .all(&self.conn)
            .await?;

        Ok(models.into_iter().map(Into::into).collect())
    }

    pub async fn delete(&self, id: i64) -> Result<(), MysqlError> {
        let result = MemoryEntity::delete_by_id(id).exec(&self.conn).await?;

        if result.rows_affected == 0 {
            return Err(MysqlError::NotFound(id));
        }

        Ok(())
    }

}
