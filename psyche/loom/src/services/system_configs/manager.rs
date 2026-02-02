use sha2::{Sha256, Digest};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set, NotSet, PaginatorTrait};
use thiserror::Error;
use time::OffsetDateTime;

use crate::services::memory::entity::MemoryEntity;
use crate::services::memory::entity::memory;
use crate::services::system_configs::entity::{Entity, SystemConfig, Column, ActiveModel};
use crate::system_configs::models::{CreateSystemConfigRequest, SystemConfigQuery, SystemConfigRecord};

#[derive(Error, Debug)]
pub enum SystemConfigError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sea_orm::DbErr),

    #[error("System config not found with id: {0}")]
    NotFound(i64),

    #[error("System config already exists with content hash: {0}")]
    AlreadyExists(String),
}

pub struct SystemConfigManager {
    conn: DatabaseConnection,
}

impl SystemConfigManager {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    /// Calculate SHA256 hash for content
    fn calculate_content_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Get the latest memory fragment ID
    pub async fn get_latest_memory_fragment_id(&self) -> Result<Option<i64>, SystemConfigError> {
        let latest_memory = MemoryEntity::find()
            .order_by_desc(memory::Column::Id)
            .one(&self.conn)
            .await?;

        Ok(latest_memory.map(|model| model.id))
    }

    /// Create a new system config record
    pub async fn create(&self, request: CreateSystemConfigRequest) -> Result<SystemConfigRecord, SystemConfigError> {
        // Calculate content hash
        let content_hash = Self::calculate_content_hash(&request.content);

        // Check if record with same hash already exists
        let existing: Option<SystemConfig> = Entity::find()
            .filter(Column::ContentHash.eq(&content_hash))
            .one(&self.conn)
            .await?;

        if existing.is_some() {
            return Err(SystemConfigError::AlreadyExists(content_hash));
        }

        // Resolve memory fragment ID if not provided
        let memory_fragment_id = match request.memory_fragment_id {
            Some(id) => Some(id),
            None => self.get_latest_memory_fragment_id().await?,
        };

        let now = OffsetDateTime::now_utc();

        // Create active model
        let active_model = ActiveModel {
            id: NotSet,
            content: Set(request.content),
            content_hash: Set(content_hash),
            memory_fragment_id: Set(memory_fragment_id),
            created_at: Set(now),
        };

        let inserted_model = active_model.insert(&self.conn).await?;
        Ok(SystemConfigRecord::from(inserted_model))
    }

    /// Query system config records
    pub async fn query(&self, query: SystemConfigQuery) -> Result<(Vec<SystemConfigRecord>, usize), SystemConfigError> {
        let mut select = Entity::find();

        // Apply filters
        if let Some(memory_fragment_id) = query.memory_fragment_id {
            select = select.filter(Column::MemoryFragmentId.eq(memory_fragment_id));
        }

        if let Some(content_hash) = query.content_hash {
            select = select.filter(Column::ContentHash.eq(content_hash));
        }

        if let Some(start_time) = query.start_time {
            select = select.filter(Column::CreatedAt.gte(start_time));
        }

        if let Some(end_time) = query.end_time {
            select = select.filter(Column::CreatedAt.lte(end_time));
        }

        // Get total count
        let total = select.clone().count(&self.conn).await?;

        // Apply ordering and pagination
        select = select.order_by_desc(Column::CreatedAt);

        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);

        let paginator = select.paginate(&self.conn, limit);
        let models: Vec<SystemConfig> = paginator.fetch_page(offset / limit).await?;

        let records = models.into_iter().map(SystemConfigRecord::from).collect();
        Ok((records, total as usize))
    }

    /// Get system config by ID
    pub async fn get_by_id(&self, id: i64) -> Result<SystemConfigRecord, SystemConfigError> {
        let model: SystemConfig = Entity::find_by_id(id)
            .one(&self.conn)
            .await?
            .ok_or(SystemConfigError::NotFound(id))?;

        Ok(SystemConfigRecord::from(model))
    }
}
