use crate::memory::types::{MemoryFragment, MemoryKind};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "memory_fragments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i64,

    #[sea_orm(column_type = "Text")]
    pub content: String,

    pub timestamp: time::OffsetDateTime,

    #[sea_orm(column_type = "Text")]
    pub kind: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<MemoryFragment> for Model {
    fn from(memory: MemoryFragment) -> Self {
        Self {
            id: memory.id,
            content: memory.content,
            timestamp: memory.timestamp,
            kind: memory.kind.as_tag().to_string(),
        }
    }
}

impl From<Model> for MemoryFragment {
    fn from(model: Model) -> Self {
        MemoryFragment {
            id: model.id,
            content: model.content,
            timestamp: model.timestamp,
            kind: MemoryKind::from_str(&model.kind),
        }
    }
}
