use async_trait::async_trait;
use epha_agent::tools::AgentTool;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::{EphemeraContext, MemoryFragmentList};
use epha_agent::context::ContextSerialize;
use loom_client::LoomClientTrait;

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

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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
                    return Err(format!("Failed to get memory {}: {}", id, e).into());
                }
            }
        }

        if fragments.is_empty() {
            Ok(format!(
                "No memories found with IDs: {}",
                args.ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")
            ))
        } else {
            // Add to context
            {
                let mut context = self.context.lock().await;
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

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: MemoryRecentArgs = serde_json::from_str(args_json)?;
        let limit = args.limit.clamp(1, 100);

        match self.loom_client.get_recent_memories(limit).await {
            Ok(response) => {
                if response.fragments.is_empty() {
                    Ok("No recent memories found.".to_string())
                } else {
                    let fragments = response.fragments.clone();

                    // Add to context
                    {
                        let mut context = self.context.lock().await;
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
            Err(e) => Err(format!("Failed to get recent memories: {}", e).into()),
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

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: MemoryTimelineArgs = serde_json::from_str(args_json)?;

        match self
            .loom_client
            .get_timeline_memory(&args.from, &args.to, args.limit, args.offset)
            .await
        {
            Ok(response) => {
                if response.fragments.is_empty() {
                    Ok(format!("No memories found between {} and {}.", args.from, args.to))
                } else {
                    let fragments = response.fragments.clone();

                    // Add to context
                    {
                        let mut context = self.context.lock().await;
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
            Err(e) => Err(format!("Failed to get memories in timeline: {}", e).into()),
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
    loom_client: Arc<dyn LoomClientTrait>,
    context: Arc<Mutex<EphemeraContext>>,
}

impl MemoryPin {
    pub fn new(
        loom_client: Arc<dyn LoomClientTrait>,
        context: Arc<Mutex<EphemeraContext>>,
    ) -> Self {
        Self { loom_client, context }
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

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: MemoryPinArgs = serde_json::from_str(args_json)?;

        // Pre-check: validate constraints before calling Loom API
        let max_count = {
            let context = self.context.lock().await;

            let already_pinned =
                context.list_pinned().iter().any(|p| p.fragment.id == args.memory_id);
            let max_count = context.max_pinned_tokens();
            let current_count = context.list_pinned().len();

            if already_pinned {
                return Err(format!("Memory {} is already pinned", args.memory_id).into());
            }

            if current_count >= max_count {
                return Err(format!(
                    "Maximum pinned count ({}) reached, please unpin some content first",
                    max_count
                )
                .into());
            }

            max_count
        };

        // Call Loom API to pin the memory
        let pinned =
            self.loom_client.pin_memory(args.memory_id, Some(args.reason.clone())).await.map_err(
                |e| -> Box<dyn std::error::Error + Send + Sync> {
                    format!("Failed to pin memory: {:?}", e).into()
                },
            )?;

        // Update local context and get new count in single lock
        let current_count = {
            let mut context = self.context.lock().await;
            context.add_pinned_memory(pinned);
            context.list_pinned().len()
        };

        Ok(format!(
            "Memory {} pinned successfully. Current pinned count: {}/{}",
            args.memory_id, current_count, max_count
        ))
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

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: MemoryUnpinArgs = serde_json::from_str(args_json)?;

        // Call Loom API to unpin the memory
        self.loom_client.unpin_memory(args.memory_id).await.map_err(
            |e| -> Box<dyn std::error::Error + Send + Sync> {
                format!("Failed to unpin memory: {:?}", e).into()
            },
        )?;

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
