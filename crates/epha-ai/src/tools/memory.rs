use anyhow::Context;
use async_trait::async_trait;
use loom_client::LoomClientTrait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::EphemeraContext;
use crate::tools::AgentTool;

// ============================================================================
// MemoryGet - Get memory fragments by IDs
// ============================================================================

#[derive(Deserialize)]
pub struct MemoryGetArgs {
    /// List of memory IDs to retrieve
    pub ids: Vec<i64>,
}

pub struct MemoryGet {
    loom_client: Arc<dyn LoomClientTrait>,
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemoryGet {
    pub fn new(
        loom_client: Arc<dyn LoomClientTrait>,
        context: Arc<Mutex<EphemeraContext>>,
    ) -> Self {
        Self { loom_client, context }
    }
}

#[async_trait]
impl AgentTool for MemoryGet {
    fn name(&self) -> &str {
        "memory_get"
    }

    fn description(&self) -> &str {
        "Retrieve specific memory fragments by their IDs. Returns the full content of each memory."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: MemoryGetArgs = serde_json::from_str(args_json)?;

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
                    return Err(e).context(format!("Failed to get memory {id}"));
                }
            }
        }

        if fragments.is_empty() {
            Ok(format!(
                "No memories found with IDs: {}",
                args.ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        } else {
            let ids: Vec<String> = fragments.iter().map(|f| f.id.to_string()).collect();
            let count = fragments.len();
            {
                let mut context = self.context.lock().await;
                context.add_recalled_memories(fragments);
            }
            Ok(format!(
                "Recalled {} memory fragments (IDs: {}). Reflect on these memories and pin any you wish to retain.",
                count,
                ids.join(", ")
            ))
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

pub struct MemoryRecent {
    loom_client: Arc<dyn LoomClientTrait>,
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemoryRecent {
    pub fn new(
        loom_client: Arc<dyn LoomClientTrait>,
        context: Arc<Mutex<EphemeraContext>>,
    ) -> Self {
        Self { loom_client, context }
    }
}

#[async_trait]
impl AgentTool for MemoryRecent {
    fn name(&self) -> &str {
        "memory_recent"
    }

    fn description(&self) -> &str {
        "Retrieve the most recent memory fragments. Use this to see what was recently remembered or to get context about recent events."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: MemoryRecentArgs = serde_json::from_str(args_json)?;
        let limit = args.limit.clamp(1, 100);

        match self.loom_client.get_recent_memories(limit).await {
            Ok(response) => {
                if response.fragments.is_empty() {
                    Ok("No recent memories found.".to_string())
                } else {
                    let count = response.fragments.len();
                    let ids: Vec<String> = response
                        .fragments
                        .iter()
                        .map(|f| f.id.to_string())
                        .collect();
                    let fragments = response.fragments.clone();
                    {
                        let mut context = self.context.lock().await;
                        context.add_recalled_memories(fragments);
                    }
                    Ok(format!(
                        "Recalled {} most recent memory fragments (IDs: {}). Reflect on these memories and pin any you wish to retain.",
                        count,
                        ids.join(", ")
                    ))
                }
            }
            Err(e) => Err(e).context("Failed to get recent memories"),
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

pub struct MemoryTimeline {
    loom_client: Arc<dyn LoomClientTrait>,
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemoryTimeline {
    pub fn new(
        loom_client: Arc<dyn LoomClientTrait>,
        context: Arc<Mutex<EphemeraContext>>,
    ) -> Self {
        Self { loom_client, context }
    }
}

#[async_trait]
impl AgentTool for MemoryTimeline {
    fn name(&self) -> &str {
        "memory_timeline"
    }

    fn description(&self) -> &str {
        "Query memory fragments within a specific time range (timeline view). Time format: ISO 8601 (e.g., '2024-01-15T10:30:00Z' or '2024-01-15T10:30:00+08:00'). Use this to retrieve memories from a specific time period."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: MemoryTimelineArgs = serde_json::from_str(args_json)?;

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
                    let count = response.fragments.len();
                    let ids: Vec<String> = response
                        .fragments
                        .iter()
                        .map(|f| f.id.to_string())
                        .collect();
                    let fragments = response.fragments.clone();
                    {
                        let mut context = self.context.lock().await;
                        context.add_recalled_memories(fragments);
                    }
                    Ok(format!(
                        "Recalled {} memory fragments from {} to {} (IDs: {}). Reflect on these memories and pin any you wish to retain.",
                        count,
                        args.from,
                        args.to,
                        ids.join(", ")
                    ))
                }
            }
            Err(e) => Err(e).context("Failed to get memories in timeline"),
        }
    }
}

// ============================================================================
// MemoryPin - Pin a memory to keep it at top of context
// ============================================================================

#[derive(Deserialize)]
pub struct MemoryPinArgs {
    /// ID of the memory to pin
    pub memory_id: i64,
    /// Why this memory should be pinned
    pub reason: String,
}

pub struct MemoryPin {
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemoryPin {
    pub fn new(context: Arc<Mutex<EphemeraContext>>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl AgentTool for MemoryPin {
    fn name(&self) -> &str {
        "memory_pin"
    }

    fn description(&self) -> &str {
        "Pin an existing memory to keep it always at the top of your context. Pinned memories persist across restarts and will not be removed by token limit management. Use this for critical information you need to remember."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "memory_id": {
                    "type": "integer",
                    "description": "ID of the memory to pin (must be an existing memory ID)"
                },
                "reason": {
                    "type": "string",
                    "description": "Why this memory should be pinned (helps you remember the purpose)"
                }
            },
            "required": ["memory_id", "reason"]
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: MemoryPinArgs = serde_json::from_str(args_json)?;

        let result = {
            let mut context = self.context.lock().await;
            context.pin(args.memory_id, args.reason).await
        };

        match result {
            Ok(()) => {
                let (usage, max) = {
                    let context = self.context.lock().await;
                    (context.pinned_token_usage(), context.max_pinned_tokens())
                };
                Ok(format!(
                    "Memory {} pinned successfully. Pinned token usage: {}/{}",
                    args.memory_id, usage, max
                ))
            }
            Err(e) => Ok(e.to_string()),
        }
    }
}

// ============================================================================
// MemoryUnpin - Remove pinned memory
// ============================================================================

#[derive(Deserialize)]
pub struct MemoryUnpinArgs {
    /// ID of the memory to unpin
    pub memory_id: i64,
}

pub struct MemoryUnpin {
    loom_client: Arc<dyn LoomClientTrait>,
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemoryUnpin {
    pub fn new(
        loom_client: Arc<dyn LoomClientTrait>,
        context: Arc<Mutex<EphemeraContext>>,
    ) -> Self {
        Self { loom_client, context }
    }
}

#[async_trait]
impl AgentTool for MemoryUnpin {
    fn name(&self) -> &str {
        "memory_unpin"
    }

    fn description(&self) -> &str {
        "Remove a pinned memory by its ID. The memory will still exist but will no longer be guaranteed to stay at the top of context."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "memory_id": {
                    "type": "integer",
                    "description": "ID of the pinned memory to remove"
                }
            },
            "required": ["memory_id"]
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: MemoryUnpinArgs = serde_json::from_str(args_json)?;

        // Call Loom API to unpin the memory
        self.loom_client
            .unpin_memory(args.memory_id)
            .await
            .context("Failed to unpin memory")?;

        // Update local context synchronously
        let removed = {
            let mut context = self.context.lock().await;
            context.remove_pinned_memory(args.memory_id)
        };

        if removed {
            Ok(format!("Memory {} unpinned successfully", args.memory_id))
        } else {
            Ok(format!(
                "Memory {} was not pinned locally (but unpinning succeeded on server)",
                args.memory_id
            ))
        }
    }
}

// ============================================================================
// Design Note: Why no MemoryListPinned tool?
// ============================================================================
//
// Pinned memories are already included in the AI's context via EphemeraContext's
// serialize() method. The AI can see all pinned memories at any time without
// needing to explicitly list them. This is by design - pinned memories are
// meant to be "always visible" context that persists across sessions.
//
// If detailed pinned info (reason, pinned_at) is needed, it can be exposed
// through the context serialization rather than a separate tool.
//
