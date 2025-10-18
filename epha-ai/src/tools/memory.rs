use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::json;
use epha_memory::{MemoryQuery, Manager, HybridMemoryManager, MemoryFragment};
use time::{OffsetDateTime, format_description};

#[derive(Deserialize)]
pub struct MemoryRecallArgs {
    pub keywords: String,
    pub query: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Memory recall error")]
pub struct MemoryRecallError;

/// Simple memory cache that stores recalled memories for selection
#[derive(Debug)]
pub struct MemoryCache {
    memories: Arc<Mutex<HashMap<i64, MemoryFragment>>>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            memories: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn store(&self, memories: Vec<MemoryFragment>) {
        let mut cache = self.memories.lock().unwrap();
        cache.clear();
        for memory in memories {
            cache.insert(memory.id, memory);
        }
    }

    pub fn get(&self, ids: &[i64]) -> Option<Vec<MemoryFragment>> {
        let cache = self.memories.lock().unwrap();
        let mut result = Vec::new();
        for &id in ids {
            if let Some(memory) = cache.get(&id) {
                result.push(memory.clone());
            } else {
                return None; // Return None if any ID not found
            }
        }
        Some(result)
    }

    pub fn clear(&self) {
        let mut cache = self.memories.lock().unwrap();
        cache.clear();
    }

    pub fn is_empty(&self) -> bool {
        let cache = self.memories.lock().unwrap();
        cache.is_empty()
    }
}

pub struct MemoryRecall {
    memory_manager: Arc<HybridMemoryManager>,
    cache: Arc<MemoryCache>,
}

impl MemoryRecall {
    pub fn new(memory_manager: Arc<HybridMemoryManager>, cache: Arc<MemoryCache>) -> Self {
        Self { memory_manager, cache }
    }

    fn format_timestamp(&self, timestamp: i64) -> String {
        let datetime = OffsetDateTime::from_unix_timestamp(timestamp)
            .unwrap_or_else(|_| OffsetDateTime::now_utc());

        let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
            .unwrap_or_else(|_| format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z").unwrap());

        datetime.format(&format)
            .unwrap_or_else(|_| timestamp.to_string())
    }

    fn serialize_memory(&self, memory: &MemoryFragment) -> String {
        format!(
            "Memory ID: {}\nCreated: {}\nSource: {}\nImportance: {}/255\nConfidence: {}/255\nTags: {}\nContent: {}",
            memory.id,
            self.format_timestamp(memory.objective_metadata.created_at),
            memory.objective_metadata.source.channel,
            memory.subjective_metadata.importance,
            memory.subjective_metadata.confidence,
            memory.subjective_metadata.tags.join(", "),
            memory.content
        )
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
            "description": "Recall relevant memories based on keywords and query. Returns detailed memory information with IDs for selection.",
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

        let recall_result = self.memory_manager.recall(&query)
            .await
            .map_err(|_| MemoryRecallError)?;

        if recall_result.memories.is_empty() {
            Ok("No relevant memories found.".to_string())
        } else {
            // Store memories in cache for selection
            self.cache.store(recall_result.memories.clone());

            // Serialize memories with full information
            let memories_text: Vec<String> = recall_result.memories
                .iter()
                .map(|m| self.serialize_memory(m))
                .collect();

            Ok(format!(
                "Recalled {} memories (cached for selection):\n\n{}\n\nUse select_memories tool to add specific memories to context.",
                recall_result.memories.len(),
                memories_text.join("\n---\n")
            ))
        }
    }
}

#[derive(Deserialize)]
pub struct SelectMemoriesArgs {
    pub memory_ids: Vec<i64>,
    pub summary: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Memory selection error")]
pub struct SelectMemoriesError;

pub struct SelectMemories {
    cache: Arc<MemoryCache>,
    context: Arc<Mutex<crate::agent::EphemeraContext>>,
}

impl SelectMemories {
    pub fn new(cache: Arc<MemoryCache>, context: Arc<Mutex<crate::agent::EphemeraContext>>) -> Self {
        Self { cache, context }
    }
}

impl Tool for SelectMemories {
    const NAME: &'static str = "select_memories";

    type Error = SelectMemoriesError;
    type Args = SelectMemoriesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "select_memories",
            "description": "Select specific memories from cache to add to context. Requires a summary explaining why these memories were selected.",
            "parameters": {
                "type": "object",
                "properties": {
                    "memory_ids": {
                        "type": "array",
                        "items": {
                            "type": "integer"
                        },
                        "description": "List of memory IDs to add to context (from recall results)"
                    },
                    "summary": {
                        "type": "string",
                        "description": "Brief summary explaining why these memories were selected and how they relate to current context"
                    }
                },
                "required": ["memory_ids", "summary"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Check if cache has memories
        if self.cache.is_empty() {
            return Err(SelectMemoriesError);
        }

        // Get memories from cache
        match self.cache.get(&args.memory_ids) {
            Some(memories) => {
                // Add memories to context
                {
                    let mut context = self.context.lock().unwrap();
                    context.add_memory_context(args.summary.clone(), memories.clone());
                }

                // Clear cache after successful selection
                let count = memories.len();
                self.cache.clear();

                Ok(format!(
                    "Successfully added {} memories to context.\nSummary: {}\n\nMemory IDs: {}",
                    count,
                    args.summary,
                    args.memory_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")
                ))
            }
            None => {
                Err(SelectMemoriesError)
            }
        }
    }
}