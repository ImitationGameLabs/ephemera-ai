use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i32,

    pub name: String,

    #[sea_orm(column_type = "Text")]
    pub bio: String,

    #[sea_orm(column_type = "Text")]
    pub password: String,

    pub message_height: i32,

    pub last_seen: Option<PrimitiveDateTime>,

    pub created_at: PrimitiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}