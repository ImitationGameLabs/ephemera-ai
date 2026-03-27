use crate::agent::{CommonPrompt, State};
use crate::context::EphemeraContext;
use crate::context::{
    ActionMemoryContent, ThoughtContent, ToChatMessages, ToolCallRecord, pending_memory,
};
use crate::sync::{SyncSender, start_sync_task};
use crate::tools::{
    MemoryGet, MemoryPin, MemoryRecent, MemoryTimeline, MemoryUnpin, StateTransition, ToolDispatch,
};
use agora_client::{AgoraClient, AgoraClientTrait};
use epha_agent::context::Context;
use epha_agent::tools::{file_system_tool_set, shell::TmuxBackend, shell_tool_set};
use llm::builder::{LLMBackend, LLMBuilder};
use llm::chat::ChatMessage;
use llm::{FunctionCall, LLMProvider, ToolCall};
use loom_client::LoomClientTrait;
use loom_client::memory::{MemoryFragment, MemoryKind};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

pub struct EphemeraAI {
    state: Arc<Mutex<State>>,
    llm: Box<dyn LLMProvider>,
    tool_dispatch: ToolDispatch,
    tool_definitions: Vec<llm::chat::Tool>,
    context: Context<EphemeraContext>,
    agora_client: Arc<dyn AgoraClientTrait>,
    config: crate::config::Config,
    sync_sender: SyncSender,
}

impl EphemeraAI {
    pub async fn new(
        config: crate::config::Config,
        loom_client: Arc<dyn LoomClientTrait>,
        http_client: reqwest::Client,
    ) -> anyhow::Result<Self> {
        // 1. Create shared state
        let state = Arc::new(Mutex::new(State::default()));

        // 2. Load common prompt
        let common_prompt = CommonPrompt::from_file("prompts/common.md")?;

        // 3. Create sync channel and context
        let (sync_sender, sync_receiver) = SyncSender::channel();

        let context_data = Arc::new(Mutex::new(EphemeraContext::new(
            loom_client.clone(),
            sync_sender.clone(),
            config.context.clone(),
        )));

        // 3.5 Start background sync task
        let _sync_handle = start_sync_task(sync_receiver, loom_client.clone());
        info!("Loom sync task started");

        // 3.6 Restore recent activities and pinned memories from Loom
        {
            let mut ctx = context_data.lock().await;
            if let Err(e) = ctx.restore_from_loom(50).await {
                tracing::warn!("Failed to restore from Loom: {}. Starting with empty context.", e);
            }
            if let Err(e) = ctx.restore_pinned_from_loom().await {
                tracing::warn!("Failed to restore pinned memories from Loom: {}", e);
            }
        }

        // 4. Initialize shell backend
        let session_name = format!("ephemera-ai-{}", Uuid::new_v4().simple());
        info!("Creating tmux session: {}", session_name);
        let backend = TmuxBackend::new(&session_name).await.map_err(|e| {
            anyhow::anyhow!("Failed to create tmux backend '{}': {}", session_name, e)
        })?;

        // 5. Initialize Agora client with health check
        let agora_client = Arc::new(AgoraClient::new(&config.agora.url, http_client));
        info!("Initializing Agora client: {}", config.agora.url);
        agora_client.health_check().await.map_err(|e| {
            anyhow::anyhow!("Agora service unavailable at '{}': {}", config.agora.url, e)
        })?;
        info!("Agora service is available");

        // 6. Build LLM provider
        let llm = LLMBuilder::new()
            .backend(LLMBackend::Groq)
            .api_key(&config.llm.api_key)
            .base_url(&config.llm.base_url)
            .model(&config.llm.model)
            .system(&common_prompt.content)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build LLM provider: {}", e))?;

        // 7. Register all tools
        let mut tool_dispatch = ToolDispatch::new();

        // Memory and state tools
        tool_dispatch.add_tool(Box::new(MemoryGet::new(loom_client.clone(), context_data.clone())));
        tool_dispatch
            .add_tool(Box::new(MemoryRecent::new(loom_client.clone(), context_data.clone())));
        tool_dispatch
            .add_tool(Box::new(MemoryTimeline::new(loom_client.clone(), context_data.clone())));
        tool_dispatch.add_tool(Box::new(MemoryPin::new(loom_client.clone(), context_data.clone())));
        tool_dispatch
            .add_tool(Box::new(MemoryUnpin::new(loom_client.clone(), context_data.clone())));
        tool_dispatch.add_tool(Box::new(StateTransition::new(state.clone())));

        // File system tools
        tool_dispatch.add_tools(file_system_tool_set());

        // Shell tools
        tool_dispatch.add_tools(shell_tool_set(Arc::new(tokio::sync::Mutex::new(backend))));

        // Pre-compute tool definitions for the LLM API
        let tool_definitions = tool_dispatch.to_llm_tools();

        Ok(Self {
            state,
            llm,
            tool_dispatch,
            tool_definitions,
            context: Context::new(context_data),
            agora_client,
            config,
            sync_sender,
        })
    }

    pub async fn live(&mut self) -> anyhow::Result<()> {
        loop {
            let state = *self.state.lock().await;
            match state {
                State::Active => {
                    // Full speed - no delay
                    self.cognitive_cycle().await?;
                }
                State::Dormant => {
                    tokio::time::sleep(Duration::from_millis(self.config.dormant_tick_interval_ms))
                        .await;
                    self.cognitive_cycle().await?;
                }
                State::Suspended => {
                    info!("Entering suspended state, exiting live loop");
                    return Ok(());
                }
            }
        }
    }

    async fn cognitive_cycle(&mut self) -> anyhow::Result<()> {
        // 1. Fetch events from Agora (POST /events/fetch)
        let events = self.agora_client.fetch_events(None).await?;

        if !events.is_empty() {
            // Collect event IDs for acknowledgment
            let event_ids: Vec<u64> = events.iter().map(|e| e.id).collect();

            // Add events to context
            self.context.data().lock().await.add_agora_events(events);

            // Acknowledge processed events
            self.agora_client.ack_events(event_ids).await?;
        }

        // 2. Build chat_history from memory (replaces context.serialize())
        let data = self.context.data();
        let ctx = data.lock().await;
        let mut chat_history: Vec<ChatMessage> = vec![];

        // 2a. Pinned memories → single user text message (reference material)
        let pinned_text = ctx.serialize_pinned();
        if let Some(ref text) = pinned_text {
            chat_history.push(ChatMessage::user().content(text.as_str()).build());
        }

        // 2b. Recent activities → ChatMessages (role-aware, ordered)
        let recent: Vec<MemoryFragment> = ctx.recent_activities().iter().cloned().collect();
        chat_history.extend(recent.iter().flat_map(|m| m.to_chat_messages()));
        drop(ctx);
        drop(pinned_text);

        // 3. Explicit multi-turn loop
        let mut current_prompt = String::new();
        let mut current_depth = 0;

        loop {
            // 3.1 Append current prompt to chat history
            if !current_prompt.is_empty() {
                chat_history.push(ChatMessage::user().content(&current_prompt).build());
            }

            // 3.2 Call LLM with tools
            let response = self
                .llm
                .chat_with_tools(&chat_history, Some(&self.tool_definitions))
                .await
                .map_err(|e| anyhow::anyhow!("LLM request failed: {}", e))?;

            // 3.3 Extract and save Thought (AI text response)
            if let Some(text) = response.text()
                && !text.is_empty()
            {
                self.save_thought(&text);
            }

            // 3.4 Extract tool calls
            let tool_calls = match response.tool_calls() {
                Some(calls) if !calls.is_empty() => calls,
                _ => break, // No tool calls -> done
            };

            // 3.5 Check depth limit
            current_depth += 1;
            if current_depth > self.config.llm.max_turns {
                warn!("Max depth {} reached, continuing next cycle", current_depth);
                break;
            }

            // 3.6 Execute tools and build results
            let mut tool_results: Vec<ToolCallRecord> = vec![];
            let mut failed_tool_name: Option<String> = None;

            for tc in &tool_calls {
                let tool_name = &tc.function.name;
                let args_str = &tc.function.arguments;

                if let Some(ref failed) = failed_tool_name {
                    tool_results.push(ToolCallRecord {
                        id: tc.id.clone(),
                        tool: tool_name.clone(),
                        args: serde_json::from_str(args_str).unwrap_or(serde_json::json!(args_str)),
                        result: format!("Skipped: tool '{}' failed earlier in this batch", failed),
                    });
                    continue;
                }

                let result = match self.tool_dispatch.call_tool(tool_name, args_str).await {
                    Ok(result) => result,
                    Err(e) => {
                        warn!("Tool '{}' failed: {}", tool_name, e);
                        tool_results.push(ToolCallRecord {
                            id: tc.id.clone(),
                            tool: tool_name.clone(),
                            args: serde_json::from_str(args_str)
                                .unwrap_or(serde_json::json!(args_str)),
                            result: format!("Error: {}", e),
                        });
                        failed_tool_name = Some(tool_name.clone());
                        continue;
                    }
                };

                tool_results.push(ToolCallRecord {
                    id: tc.id.clone(),
                    tool: tool_name.clone(),
                    args: serde_json::from_str(args_str).unwrap_or(serde_json::json!(args_str)),
                    result,
                });
            }

            // Save Action memory (one per LLM response, not per tool call)
            self.save_action(&tool_results);

            // 3.7 Update chat history for next iteration
            // Add assistant message with tool calls
            chat_history
                .push(ChatMessage::assistant().tool_use(tool_calls.clone()).content("").build());

            // Add tool result message
            // The llm crate's OpenAI compatible provider expands ToolResult into
            // separate "tool" role messages using ToolCall.id as tool_call_id
            // and ToolCall.function.arguments as the content.
            let result_tool_calls: Vec<ToolCall> = tool_results
                .iter()
                .map(|r| ToolCall {
                    id: r.id.clone(),
                    call_type: "function".to_string(),
                    function: FunctionCall { name: r.tool.clone(), arguments: r.result.clone() },
                })
                .collect();

            chat_history
                .push(ChatMessage::user().tool_result(result_tool_calls).content("").build());

            // 3.8 Clear prompt for subsequent iterations
            current_prompt = String::new();
        }

        Ok(())
    }

    /// Save a Thought memory (AI's text response)
    fn save_thought(&self, text: &str) {
        let content = serde_json::to_string(&ThoughtContent { text: text.to_string() }).unwrap();
        let fragment = pending_memory(content, MemoryKind::Thought);
        self.sync_sender.send(fragment);
    }

    /// Save an Action memory (all tool calls and results from one LLM response)
    fn save_action(&self, tool_results: &[ToolCallRecord]) {
        let content = ActionMemoryContent { tool_calls: tool_results.to_vec() };
        let content = serde_json::to_string(&content).unwrap();
        let fragment = pending_memory(content, MemoryKind::Action);
        self.sync_sender.send(fragment);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_action_memory_serialization() {
        let records = vec![
            ToolCallRecord {
                id: "call_abc123".to_string(),
                tool: "memory_get".to_string(),
                args: serde_json::json!({"key": "recent"}),
                result: "Found 3 memories".to_string(),
            },
            ToolCallRecord {
                id: "call_def456".to_string(),
                tool: "shell_exec".to_string(),
                args: serde_json::json!({"command": "ls"}),
                result: "file1.txt\nfile2.txt".to_string(),
            },
            ToolCallRecord {
                id: "call_skip789".to_string(),
                tool: "file_read".to_string(),
                args: serde_json::json!({"path": "/tmp/x"}),
                result: "Skipped: tool 'shell_exec' failed earlier in this batch".to_string(),
            },
        ];

        let content = ActionMemoryContent { tool_calls: records };
        let json = serde_json::to_string_pretty(&content).unwrap();
        println!("{}", json);

        // Verify round-trip
        let deserialized: ActionMemoryContent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tool_calls.len(), 3);
        assert_eq!(deserialized.tool_calls[0].id, "call_abc123");
        assert_eq!(deserialized.tool_calls[1].args["command"], "ls");
        assert!(deserialized.tool_calls[2].result.contains("Skipped"));
    }
}
