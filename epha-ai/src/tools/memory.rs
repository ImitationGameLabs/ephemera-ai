use std::sync::Arc;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::json;
use epha_memory::{MemoryQuery, Manager, HybridMemoryManager};

#[derive(Deserialize)]
pub struct MemoryRecallArgs {
    pub keywords: String,
    pub query: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Memory recall error")]
pub struct MemoryRecallError;

pub struct MemoryRecall {
    memory_manager: Arc<HybridMemoryManager>,
}

impl MemoryRecall {
    pub fn new(memory_manager: Arc<HybridMemoryManager>) -> Self {
        Self { memory_manager }
    }
}

impl Tool for MemoryRecall {
    const NAME: &'static str = "memory_recall";

    type Error = MemoryRecallError;
    type Args = MemoryRecallArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "memory_recall",
            "description": "Recall relevant memories based on keywords and query",
            "parameters": {
                "type": "object",
                "properties": {
                    "keywords": {
                        "type": "string",
                        "description": "Keywords to search for in memories (space-separated)"
                    },
                    "query": {
                        "type": "string",
                        "description": "Natural language query describing what to recall"
                    }
                },
                "required": ["keywords", "query"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let query = MemoryQuery {
            keywords: args.keywords,
            time_range: None,
        };

        // Clone the needed data to avoid holding the lock across await
        // let memory_manager_clone = self.memory_manager.clone();
        let recall_result = self.memory_manager.recall(&query)
            .await
            .map_err(|_| MemoryRecallError)?;

        if recall_result.memories.is_empty() {
            Ok("No relevant memories found.".to_string())
        } else {
            let memories_text: Vec<String> = recall_result.memories
                .iter()
                .map(|m| format!("- [{}] {}", m.objective_metadata.created_at, m.content))
                .collect();
            Ok(format!("Recalled {} memories:\n{}", recall_result.memories.len(), memories_text.join("\n")))
        }
    }
}