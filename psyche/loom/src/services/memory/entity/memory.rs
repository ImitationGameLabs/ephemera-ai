use crate::memory::types::{MemoryFragment, MemorySource};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "memory_fragments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    #[sea_orm(column_type = "Text")]
    pub content: String,

    #[sea_orm(column_name = "created_at", column_type = "custom(\"DATETIME(6)\")")]
    pub timestamp: time::OffsetDateTime,

    #[sea_orm(column_type = "Text")]
    pub source: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<MemoryFragment> for Model {
    fn from(memory: MemoryFragment) -> Self {
        Self {
            id: 0, // This will be ignored by insert
            content: memory.content,
            timestamp: memory.timestamp,
            source: serde_json::to_string(&memory.source)
                .expect("MemorySource must be serializable"),
        }
    }
}

impl From<Model> for MemoryFragment {
    fn from(model: Model) -> Self {
        let source: MemorySource =
            serde_json::from_str(&model.source).expect("Source must be valid MemorySource JSON");

        MemoryFragment {
            id: model.id,
            content: model.content,
            timestamp: model.timestamp,
            source,
        }
    }
}
