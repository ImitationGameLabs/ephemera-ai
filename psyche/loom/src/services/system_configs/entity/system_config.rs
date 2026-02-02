use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "system_configs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    #[sea_orm(column_type = "Text")]
    pub content: String,

    pub content_hash: String,

    pub memory_fragment_id: Option<i64>,

    #[sea_orm(column_type = "custom(\"DATETIME(6)\")")]
    pub created_at: time::OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<crate::system_configs::models::SystemConfigRecord> for Model {
    fn from(record: crate::system_configs::models::SystemConfigRecord) -> Self {
        Self {
            id: record.id,
            content: record.content,
            content_hash: record.content_hash,
            memory_fragment_id: record.memory_fragment_id,
            created_at: record.created_at,
        }
    }
}

impl From<Model> for crate::system_configs::models::SystemConfigRecord {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            content: model.content,
            content_hash: model.content_hash,
            memory_fragment_id: model.memory_fragment_id,
            created_at: model.created_at,
        }
    }
}