
use anyhow::Ok;
use meilisearch_sdk::{client::*, indexes::Index};
// use sea_orm::Database;

use crate::{Manager, MemoryFragment, MemorySource, ObjectiveMetadata, QueryArgs, QueryResult, SubjectiveMetadata};

const MEILI_INDEX_NAME: &'static str = "ephemera-memory";

pub struct MeiliMemoryManager {
    // meili_client: Client,
    meili_index: Index,

    // db: Database,
}

impl MeiliMemoryManager {
    pub fn new(
        meili_client: Client, 
        // db: Database,
    ) -> Self {
        let meili_index = meili_client.index(MEILI_INDEX_NAME);

        Self { 
            // meili_client,
            meili_index,
            // db,
        }
    }

    pub fn fill_objective_metadata(&self, memory: &mut MemoryFragment) {
        let now: time::UtcDateTime = time::UtcDateTime::now();
        let timestamp = now.unix_timestamp();
        let milliseconds = timestamp * 1000 + now.millisecond() as i64;
        memory.id = milliseconds;
        memory.objective_metadata.created_at = timestamp;
    }

    pub fn from_content(&self, content: String) -> MemoryFragment {
        let now: time::UtcDateTime = time::UtcDateTime::now();
        let timestamp = now.unix_timestamp();
        let milliseconds = timestamp * 1000 + now.millisecond() as i64;

        MemoryFragment { 
            id: milliseconds, 
            content, 
            subjective_metadata: SubjectiveMetadata::default(), 
            objective_metadata: ObjectiveMetadata {
                created_at: timestamp,
                source: MemorySource::Unknown,
            }, 
            associations: Vec::new(),
        }
    }
}

impl Manager for MeiliMemoryManager {
    async fn append(&mut self, memory: &MemoryFragment) -> anyhow::Result<()> {
        // TODO: Save to DB
        // self.meili_index.fetch_info().await?;

        self.meili_index.add_documents(&[memory], Some("id")).await?;
        Ok(())
    }

    async fn recall(&self, query: &QueryArgs) -> anyhow::Result<QueryResult> {
        let memorys = self.meili_index.search()
            .with_limit(10)
            .with_query(&query.keywords)
            .execute::<MemoryFragment>()
            .await
            .unwrap();

        let mems = memorys.hits
            .iter()
            .map(|hit| hit.result.clone())
            .collect();

        Ok(QueryResult { memories: mems })
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    // use super::*;

    #[test]
    fn test_print_timestamp() {
        let now: time::UtcDateTime = time::UtcDateTime::now();
        println!("seconds {}", now.unix_timestamp());

        let milliseconds = now.unix_timestamp() * 1000 + now.millisecond() as i64;
        println!("milliseconds {}", milliseconds)
    }
}