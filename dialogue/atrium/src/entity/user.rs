use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub name: String,

    #[sea_orm(column_type = "Text")]
    pub bio: String,

    #[sea_orm(column_type = "Text")]
    pub password: String,

    pub message_height: i32,

    pub last_seen: Option<OffsetDateTime>,

    pub created_at: OffsetDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}