use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, NotSet, QueryOrder, QuerySelect};
use thiserror::Error;
use time::{OffsetDateTime, PrimitiveDateTime};

use crate::entity::{MessageEntity};
use crate::entity::message;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sea_orm::DbErr),

    #[error("Message not found with id: {0}")]
    MessageNotFound(i32),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct MessageDto {
    pub id: i32,
    pub content: String,
    pub sender: String,
    pub created_at: PrimitiveDateTime,
}

impl From<message::Model> for MessageDto {
    fn from(model: message::Model) -> Self {
        Self {
            id: model.id,
            content: model.content,
            sender: model.sender,
            created_at: model.created_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateMessageDto {
    pub content: String,
    pub sender: String,
}

#[derive(Clone)]
pub struct MessageManager {
    pub conn: DatabaseConnection,
}

impl MessageManager {
    pub fn new(conn: DatabaseConnection) -> Self {
        Self { conn }
    }

    pub async fn create_message(&self, message_dto: &CreateMessageDto) -> Result<MessageDto, DbError> {
        let now = PrimitiveDateTime::new(OffsetDateTime::now_utc().date(), OffsetDateTime::now_utc().time());
        let active_model = message::ActiveModel {
            id: NotSet,
            content: Set(message_dto.content.clone()),
            sender: Set(message_dto.sender.clone()),
            created_at: Set(now),
        };

        let inserted_model = active_model.insert(&self.conn).await?;
        Ok(inserted_model.into())
    }

    pub async fn get_message(&self, id: i32) -> Result<MessageDto, DbError> {
        let model = MessageEntity::find_by_id(id)
            .one(&self.conn)
            .await?
            .ok_or(DbError::MessageNotFound(id))?;

        Ok(model.into())
    }

    pub async fn get_messages(
        &self,
        sender_filter: Option<&str>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<MessageDto>, DbError> {
        let mut query = MessageEntity::find();

        if let Some(sender) = sender_filter {
            query = query.filter(message::Column::Sender.eq(sender));
        }

        query = query.order_by_desc(message::Column::CreatedAt);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(offset) = offset {
            query = query.offset(offset);
        }

        let models = query.all(&self.conn).await?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    pub async fn delete_message(&self, id: i32) -> Result<(), DbError> {
        let result = MessageEntity::delete_by_id(id).exec(&self.conn).await?;

        if result.rows_affected == 0 {
            return Err(DbError::MessageNotFound(id));
        }

        Ok(())
    }

    pub async fn get_latest_message_id(&self) -> Result<Option<i32>, DbError> {
        let latest_message = MessageEntity::find()
            .order_by_desc(message::Column::Id)
            .one(&self.conn)
            .await?;

        Ok(latest_message.map(|m| m.id))
    }

    pub async fn get_messages_since_id(
        &self,
        since_id: i32,
        limit: Option<u64>,
    ) -> Result<Vec<MessageDto>, DbError> {
        let mut query = MessageEntity::find()
            .filter(message::Column::Id.gt(since_id))
            .order_by_asc(message::Column::Id);

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        let models = query.all(&self.conn).await?;
        Ok(models.into_iter().map(Into::into).collect())
    }
}