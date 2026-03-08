use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "pinned")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub memory_id: i64,
    pub reason: Option<String>,
    pub pinned_at: time::OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::memory::Entity",
        from = "Column::MemoryId",
        to = "super::memory::Column::Id"
    )]
    Memory,
}

impl Related<super::memory::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Memory.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
