use crate::context::{EphemeraContext, MemoryFragmentList};
use epha_agent::context::ContextSerialize;
use loom_client::LoomClient;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::json;
use std::sync::{Arc, Mutex};

// ============================================================================
// MemoryGet - Get memory fragments by IDs
// ============================================================================

#[derive(Deserialize)]
pub struct MemoryGetArgs {
    /// List of memory IDs to retrieve
    pub ids: Vec<i64>,
}

#[derive(Debug, thiserror::Error)]
#[error("Memory get error: {0}")]
pub struct MemoryGetError(String);

pub struct MemoryGet {
    loom_client: Arc<LoomClient>,
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemoryGet {
    pub fn new(loom_client: Arc<LoomClient>, context: Arc<Mutex<EphemeraContext>>) -> Self {
        Self {
            loom_client,
            context,
        }
    }
}

impl Tool for MemoryGet {
    const NAME: &'static str = "memory_get";

    type Error = MemoryGetError;
    type Args = MemoryGetArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "memory_get",
            "description": "Retrieve specific memory fragments by their IDs. Returns the full content of each memory.",
            "parameters": {
                "type": "object",
                "properties": {
                    "ids": {
                        "type": "array",
                        "items": {
                            "type": "integer"
                        },
                        "description": "List of memory IDs to retrieve"
                    }
                },
                "required": ["ids"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.ids.is_empty() {
            return Ok("No memory IDs provided.".to_string());
        }

        let mut fragments = Vec::new();
        for id in &args.ids {
            match self.loom_client.get_memory(*id).await {
                Ok(response) => {
                    if let Some(fragment) = response.first() {
                        fragments.push(fragment.clone());
                    }
                }
                Err(e) => {
                    return Err(MemoryGetError(format!(
                        "Failed to get memory {}: {}",
                        id, e
                    )));
                }
            }
        }

        if fragments.is_empty() {
            Ok(format!(
                "No memories found with IDs: {}",
                args.ids.iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        } else {
            // Add to context
            {
                let mut context = self.context.lock().unwrap();
                context.add_memory_context(
                    format!("Retrieved {} memories by ID", fragments.len()),
                    fragments.clone(),
                );
            }

            let serialized = MemoryFragmentList::from(fragments).serialize();
            Ok(format!("Retrieved {} memories:\n\n{}", args.ids.len(), serialized))
        }
    }
}

// ============================================================================
// MemoryRecent - Get recent memory fragments
// ============================================================================

#[derive(Deserialize)]
pub struct MemoryRecentArgs {
    /// Maximum number of recent memories to retrieve (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, thiserror::Error)]
#[error("Memory recent error: {0}")]
pub struct MemoryRecentError(String);

pub struct MemoryRecent {
    loom_client: Arc<LoomClient>,
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemoryRecent {
    pub fn new(loom_client: Arc<LoomClient>, context: Arc<Mutex<EphemeraContext>>) -> Self {
        Self {
            loom_client,
            context,
        }
    }
}

impl Tool for MemoryRecent {
    const NAME: &'static str = "memory_recent";

    type Error = MemoryRecentError;
    type Args = MemoryRecentArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "memory_recent",
            "description": "Retrieve the most recent memory fragments. Use this to see what was recently remembered or to get context about recent events.",
            "parameters": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of recent memories to retrieve (default: 10)",
                        "minimum": 1,
                        "maximum": 100
                    }
                },
                "required": []
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, MemoryRecentError> {
        let limit = args.limit.max(1).min(100);

        match self.loom_client.get_recent_memory(limit).await {
            Ok(response) => {
                if response.fragments.is_empty() {
                    Ok("No recent memories found.".to_string())
                } else {
                    let fragments = response.fragments.clone();

                    // Add to context
                    {
                        let mut context = self.context.lock().unwrap();
                        context.add_memory_context(
                            format!("Retrieved {} most recent memories", fragments.len()),
                            fragments.clone(),
                        );
                    }

                    let serialized = MemoryFragmentList::from(fragments).serialize();
                    Ok(format!(
                        "Retrieved {} most recent memories:\n\n{}",
                        response.fragments.len(),
                        serialized
                    ))
                }
            }
            Err(e) => Err(MemoryRecentError(format!("Failed to get recent memories: {}", e))),
        }
    }
}

// ============================================================================
// MemoryTimeline - Get memory fragments within a time range (timeline view)
// ============================================================================

#[derive(Deserialize)]
pub struct MemoryTimelineArgs {
    /// Start time in ISO 8601 format (e.g., "2024-01-15T10:30:00Z" or "2024-01-15T10:30:00+08:00")
    pub from: String,
    /// End time in ISO 8601 format
    pub to: String,
    /// Maximum number of memories to return
    pub limit: Option<usize>,
    /// Number of memories to skip (for pagination)
    pub offset: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
#[error("Memory timeline error: {0}")]
pub struct MemoryTimelineError(String);

pub struct MemoryTimeline {
    loom_client: Arc<LoomClient>,
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemoryTimeline {
    pub fn new(loom_client: Arc<LoomClient>, context: Arc<Mutex<EphemeraContext>>) -> Self {
        Self {
            loom_client,
            context,
        }
    }
}

impl Tool for MemoryTimeline {
    const NAME: &'static str = "memory_timeline";

    type Error = MemoryTimelineError;
    type Args = MemoryTimelineArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "memory_timeline",
            "description": "Query memory fragments within a specific time range (timeline view). Time format: ISO 8601 (e.g., '2024-01-15T10:30:00Z' or '2024-01-15T10:30:00+08:00'). Use this to retrieve memories from a specific time period.",
            "parameters": {
                "type": "object",
                "properties": {
                    "from": {
                        "type": "string",
                        "description": "Start time in ISO 8601 format (e.g., '2024-01-01T00:00:00Z')"
                    },
                    "to": {
                        "type": "string",
                        "description": "End time in ISO 8601 format (e.g., '2024-12-31T23:59:59Z')"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of memories to return (default: no limit)",
                        "minimum": 1
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Number of memories to skip for pagination (default: 0)",
                        "minimum": 0
                    }
                },
                "required": ["from", "to"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, MemoryTimelineError> {
        match self
            .loom_client
            .get_timeline_memory(&args.from, &args.to, args.limit, args.offset)
            .await
        {
            Ok(response) => {
                if response.fragments.is_empty() {
                    Ok(format!(
                        "No memories found between {} and {}.",
                        args.from, args.to
                    ))
                } else {
                    let fragments = response.fragments.clone();

                    // Add to context
                    {
                        let mut context = self.context.lock().unwrap();
                        context.add_memory_context(
                            format!(
                                "Retrieved {} memories from {} to {}",
                                fragments.len(),
                                args.from,
                                args.to
                            ),
                            fragments.clone(),
                        );
                    }

                    let serialized = MemoryFragmentList::from(fragments).serialize();
                    Ok(format!(
                        "Retrieved {} memories from {} to {}:\n\n{}",
                        response.fragments.len(),
                        args.from,
                        args.to,
                        serialized
                    ))
                }
            }
            Err(e) => Err(MemoryTimelineError(format!(
                "Failed to get memories in timeline: {}",
                e
            ))),
        }
    }
}
