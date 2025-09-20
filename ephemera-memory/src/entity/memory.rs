use crate::{MemoryFragment, MemorySource, ObjectiveMetadata, Speaker, SubjectiveMetadata};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "memory_fragments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    #[sea_orm(column_type = "Text")]
    pub content: String,

    pub created_at: i64,

    #[sea_orm(column_type = "Text")]
    pub source: String,

    pub importance: i32,
    pub confidence: i32,

    #[sea_orm(column_type = "Text")]
    pub tags: String,

    #[sea_orm(column_type = "Text")]
    pub notes: String,

    #[sea_orm(column_type = "Text")]
    pub associations: String,

    #[sea_orm(column_type = "Text")]
    pub claimed_identity: String,

    #[sea_orm(column_type = "Text")]
    pub assessed_identity: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl From<MemoryFragment> for Model {
    fn from(memory: MemoryFragment) -> Self {
        let (claimed_identity, assessed_identity) = match &memory.objective_metadata.source {
            MemorySource::StatementByOther(speaker) => (
                speaker.claimed_identity.clone(),
                speaker.assessed_identity.clone(),
            ),
            _ => (String::new(), String::new()),
        };

        Self {
            id: memory.id,
            content: memory.content,
            created_at: memory.objective_metadata.created_at,
            source: format!("{:?}", memory.objective_metadata.source),
            importance: memory.subjective_metadata.importance as i32,
            confidence: memory.subjective_metadata.confidence as i32,
            tags: serde_json::to_string(&memory.subjective_metadata.tags).unwrap_or_default(),
            notes: memory.subjective_metadata.notes,
            associations: serde_json::to_string(&memory.associations).unwrap_or_default(),
            claimed_identity,
            assessed_identity,
        }
    }
}

impl From<Model> for MemoryFragment {
    fn from(model: Model) -> Self {
        let source = match model.source.as_str() {
            "Reflection" => MemorySource::Reflection,
            "Internet" => MemorySource::Internet,
            "StatementBySelf" => MemorySource::StatementBySelf,
            _ if model.source.starts_with("StatementByOther") => {
                MemorySource::StatementByOther(Speaker {
                    claimed_identity: model.claimed_identity,
                    assessed_identity: model.assessed_identity,
                })
            }
            _ => MemorySource::Unknown,
        };

        MemoryFragment {
            id: model.id,
            content: model.content,
            subjective_metadata: SubjectiveMetadata {
                importance: model.importance as u8,
                confidence: model.confidence as u8,
                tags: serde_json::from_str(&model.tags).unwrap_or_default(),
                notes: model.notes,
            },
            objective_metadata: ObjectiveMetadata {
                created_at: model.created_at,
                source,
            },
            associations: serde_json::from_str(&model.associations).unwrap_or_default(),
        }
    }
}
