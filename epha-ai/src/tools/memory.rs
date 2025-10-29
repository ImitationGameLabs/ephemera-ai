use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::json;
use loom_client::{LoomClient, SearchMemoryRequest, MemoryResponse};
use loom_client::memory::MemoryFragment;
use epha_agent::context::ContextSerialize;
use crate::context::{MemoryFragmentList, EphemeraContext};

#[derive(Deserialize)]
pub struct MemoryRecallArgs {
    pub keywords: String,
    pub query: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Memory recall error")]
pub struct MemoryRecallError;

/// Simple memory cache that stores recalled memories for selection
#[derive(Debug, Default)]
pub struct RecallCacheHelper {
    memories: HashMap<i64, MemoryFragment>,
}

impl RecallCacheHelper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn store(&mut self, memories: Vec<MemoryFragment>) {
        self.memories.clear();
        for memory in memories {
            self.memories.insert(memory.id, memory);
        }
    }

    pub fn store_responses(&mut self, responses: Vec<MemoryResponse>) {
        self.memories.clear();
        for response in responses {
            // Convert MemoryResponse to MemoryFragment for compatibility
            let fragment: MemoryFragment = MemoryFragment {
                id: response.id,
                content: response.content,
                subjective_metadata: Default::default(), // Will use default values
                objective_metadata: loom_client::memory::ObjectiveMetadata {
                    created_at: response.created_at.unix_timestamp(),
                    source: response.source
                        .map(|s| loom_client::memory::MemorySource::action(s))
                        .unwrap_or_else(|| loom_client::memory::MemorySource::action("unknown".to_string())),
                },
                associations: Vec::new(),
            };
            self.memories.insert(response.id, fragment);
        }
    }

    pub fn get(&self, ids: &[i64]) -> Option<Vec<MemoryFragment>> {
        let mut result = Vec::new();
        for &id in ids {
            if let Some(memory) = self.memories.get(&id) {
                result.push(memory.clone());
            } else {
                return None; // Return None if any ID not found
            }
        }
        Some(result)
    }

    pub fn clear(&mut self) {
        self.memories.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.memories.is_empty()
    }
}

pub struct MemoryRecall {
    loom_client: Arc<LoomClient>,
    cache: Arc<Mutex<RecallCacheHelper>>,
}

impl MemoryRecall {
    pub fn new(loom_client: Arc<LoomClient>, cache: Arc<Mutex<RecallCacheHelper>>) -> Self {
        Self { loom_client, cache }
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
        let search_request = SearchMemoryRequest {
            keywords: args.keywords,
            start_time: None,
            end_time: None,
        };

        let search_result = self.loom_client.search_memory(search_request)
            .await
            .map_err(|_| MemoryRecallError)?;

        if search_result.memories.is_empty() {
            Ok("No relevant memories found.".to_string())
        } else {
            // Store memory responses in cache for selection
            self.cache.lock().unwrap().store_responses(search_result.memories.clone());

            // Convert responses to fragments for serialization
            let fragments: Vec<MemoryFragment> = search_result.memories.into_iter()
                .map(|response| MemoryFragment {
                    id: response.id,
                    content: response.content,
                    subjective_metadata: Default::default(),
                    objective_metadata: loom_client::memory::ObjectiveMetadata {
                        created_at: response.created_at.unix_timestamp(),
                        source: response.source
                            .map(|s| loom_client::memory::MemorySource::action(s))
                            .unwrap_or_else(|| loom_client::memory::MemorySource::action("unknown".to_string())),
                    },
                    associations: Vec::new(),
                })
                .collect();

            // Use unified serialization
            let serialized_memories = MemoryFragmentList::from(fragments.clone()).serialize();

            Ok(format!(
                "Recalled {} memories (cached for selection):\n\n{}\n\nUse select_memories tool to add specific memories to context.",
                fragments.len(),
                serialized_memories
            ))
        }
    }
}

#[derive(Deserialize)]
pub struct MemorySelectionArgs {
    pub memory_ids: Vec<i64>,
    pub summary: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Memory selection error")]
pub struct MemorySelectionError;

pub struct MemorySelection {
    cache: Arc<Mutex<RecallCacheHelper>>,
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemorySelection {
    pub fn new(cache: Arc<Mutex<RecallCacheHelper>>, context: Arc<Mutex<EphemeraContext>>) -> Self {
        Self { cache, context }
    }
}

impl Tool for MemorySelection {
    const NAME: &'static str = "select_memories";

    type Error = MemorySelectionError;
    type Args = MemorySelectionArgs;
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
        {
            let cache = self.cache.lock().unwrap();
            if cache.is_empty() {
                return Err(MemorySelectionError);
            }
        } // Release lock before get operation

        // Get memories from cache
        match self.cache.lock().unwrap().get(&args.memory_ids) {
            Some(memories) => {
                // Add memories to context
                {
                    let mut context = self.context.lock().unwrap();
                    context.add_memory_context(args.summary.clone(), memories.clone());
                }

                // Clear cache after successful selection
                let count = memories.len();
                self.cache.lock().unwrap().clear();

                Ok(format!(
                    "Successfully added {} memories to context.\nSummary: {}\n\nMemory IDs: {}",
                    count,
                    args.summary,
                    args.memory_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")
                ))
            }
            None => {
                Err(MemorySelectionError)
            }
        }
    }
}