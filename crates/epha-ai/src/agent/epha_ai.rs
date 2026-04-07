use crate::agent::{CommonPrompt, State};
use crate::context::EphemeraContext;
use crate::context::{
    ActionMemoryContent, ThoughtContent, ToChatMessages, TokenBudgetError, ToolCallRecord,
    pending_memory, serialize_recalled_xml,
};
use crate::sync::{SyncSender, start_sync_task};
use crate::tools::shell::{TmuxBackend, shell_tool_set};
use crate::tools::{
    ContextEvict, MemoryGet, MemoryPin, MemoryRecent, MemoryTimeline, MemoryUnpin, StateTransition,
    ToolDispatch,
};
use agora_client::{AgoraClient, AgoraClientTrait};
use llm::builder::{LLMBackend, LLMBuilder};
use llm::chat::ChatMessage;
use llm::{FunctionCall, LLMProvider, ToolCall};
use loom_client::LoomClientTrait;
use loom_client::memory::{MemoryFragment, MemoryKind};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Try to parse a JSON string into a `serde_json::Value`.
///
/// If the string is valid JSON (e.g. `{"key": "value"}`), `from_str` produces
/// the corresponding object/array. If it is malformed, `json!(s)` wraps the raw
/// string as a JSON string value so that it can still be stored without losing information.
fn preserve_raw_json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap_or(serde_json::json!(s))
}

/// Estimate the static token overhead from system prompt and tool definitions.
fn compute_static_overhead(system_prompt: &str, tool_definitions: &[llm::chat::Tool]) -> usize {
    let json = serde_json::json!({
        "messages": [{"role": "system", "content": system_prompt}],
        "tools": tool_definitions,
    });
    tokenx_rs::estimate_token_count(&serde_json::to_string(&json).unwrap())
}

pub struct EphemeraAI {
    state: Arc<Mutex<State>>,
    llm: Box<dyn LLMProvider>,
    tool_dispatch: ToolDispatch,
    tool_definitions: Vec<llm::chat::Tool>,
    context: Arc<Mutex<EphemeraContext>>,
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
                warn!(
                    "Failed to restore from Loom: {}. Starting with empty context.",
                    e
                );
            }
            if let Err(e) = ctx.restore_pinned_from_loom().await {
                warn!("Failed to restore pinned memories from Loom: {}", e);
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
        tool_dispatch.add_tool(Box::new(MemoryGet::new(
            loom_client.clone(),
            context_data.clone(),
        )));
        tool_dispatch.add_tool(Box::new(MemoryRecent::new(
            loom_client.clone(),
            context_data.clone(),
        )));
        tool_dispatch.add_tool(Box::new(MemoryTimeline::new(
            loom_client.clone(),
            context_data.clone(),
        )));
        tool_dispatch.add_tool(Box::new(MemoryPin::new(
            loom_client.clone(),
            context_data.clone(),
        )));
        tool_dispatch.add_tool(Box::new(MemoryUnpin::new(
            loom_client.clone(),
            context_data.clone(),
        )));
        tool_dispatch.add_tool(Box::new(StateTransition::new(state.clone())));
        tool_dispatch.add_tool(Box::new(ContextEvict::new(context_data.clone())));

        // Shell tools
        tool_dispatch.add_tools(shell_tool_set(Arc::new(tokio::sync::Mutex::new(backend))));

        // Pre-compute tool definitions for the LLM API
        let tool_definitions = tool_dispatch.to_llm_tools();

        // Compute static token overhead (system prompt + tool definitions)
        let static_overhead = compute_static_overhead(&common_prompt.content, &tool_definitions);
        info!(
            "Static token overhead: {} (system prompt + tool definitions)",
            static_overhead
        );

        {
            let mut ctx = context_data.lock().await;
            ctx.set_static_overhead(static_overhead);
        }

        Ok(Self {
            state,
            llm,
            tool_dispatch,
            tool_definitions,
            context: context_data,
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
            self.context.lock().await.add_agora_events(events);

            // Acknowledge processed events
            self.agora_client.ack_events(event_ids).await?;
        }

        // 2. Build chat_history from memory (replaces context.serialize())
        let data = self.context.clone();
        let ctx = data.lock().await;
        let mut chat_history: Vec<ChatMessage> = vec![];

        // 2a. Pinned memories → single assistant text message (reference material)
        let pinned_text = ctx.serialize_pinned();
        if let Some(ref text) = pinned_text {
            chat_history.push(ChatMessage::assistant().content(text.as_str()).build());
        }

        // 2b. Recent activities → ChatMessages (role-aware, ordered)
        let recent: Vec<MemoryFragment> = ctx.recent_activities().iter().cloned().collect();
        chat_history.extend(recent.iter().flat_map(|m| m.to_chat_messages()));

        // Release the lock before entering the agent loop
        drop(ctx);

        // 3. Explicit multi-turn loop
        let mut current_prompt = String::new();
        let mut current_depth = 0;
        // Only recalled memories and tool results need explicit tracking here —
        // tool definitions are already excluded via effective_ceiling, and
        // response_reserve_tokens absorbs the small per-turn overhead (user
        // prompts, assistant tool_use wrappers).
        let mut remaining_budget = self.context.lock().await.available_budget();

        loop {
            // 3.1 Append current prompt to chat history
            if !current_prompt.is_empty() {
                chat_history.push(ChatMessage::user().content(&current_prompt).build());
            }

            // 3.1.1 Inject recalled memories with budget gate
            {
                let mut ctx = self.context.lock().await;
                if !ctx.recalled_memories().is_empty() {
                    let recall_text = serialize_recalled_xml(ctx.recalled_memories());
                    let recall_tokens = tokenx_rs::estimate_token_count(&recall_text);
                    if recall_tokens <= remaining_budget {
                        chat_history.push(ChatMessage::user().content(&recall_text).build());
                        remaining_budget -= recall_tokens;
                    } else {
                        let error = TokenBudgetError {
                            requested_tokens: recall_tokens,
                            available_tokens: remaining_budget,
                        };
                        chat_history.push(ChatMessage::user().content(error.to_string()).build());
                    }
                    ctx.clear_recalled_memories();
                }
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
                        args: preserve_raw_json(args_str),
                        result: format!("Skipped: tool '{}' failed earlier in this batch", failed),
                    });
                    continue;
                }

                let result = match self.tool_dispatch.call_tool(tool_name, args_str).await {
                    Ok(result) => result,
                    Err(e) => {
                        // Err means system-level failure (network, serialization, etc.),
                        // not a business-logic failure — those are returned as Ok(String).
                        // e already carries tool name and full error chain from dispatch;
                        // args is appended here since dispatch has no access to it.
                        error!("Tool failed (args: {}): {}", args_str, e);
                        tool_results.push(ToolCallRecord {
                            id: tc.id.clone(),
                            tool: tool_name.clone(),
                            args: preserve_raw_json(args_str),
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

            // 3.7 Budget gate on tool results
            for record in &mut tool_results {
                let tokens = tokenx_rs::estimate_token_count(&record.result);
                if tokens <= remaining_budget {
                    remaining_budget -= tokens;
                } else {
                    record.result = TokenBudgetError {
                        requested_tokens: tokens,
                        available_tokens: remaining_budget,
                    }
                    .to_string();
                }
            }

            // 3.7.1 Update chat history for next iteration
            // Add assistant message with tool calls
            chat_history.push(
                ChatMessage::assistant()
                    .tool_use(tool_calls.clone())
                    .content("")
                    .build(),
            );

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

            chat_history.push(
                ChatMessage::user()
                    .tool_result(result_tool_calls)
                    .content("")
                    .build(),
            );

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

    // ---------------------------------------------------------------------------
    // Visual render test for context serialization output.
    // Intentionally has no assertions — run with `--ignored` to inspect
    // the full serialized chat history and verify context rendering.
    // ---------------------------------------------------------------------------

    #[test]
    #[ignore]
    fn render_chat_history() {
        use crate::config::ContextConfig;
        use crate::context::{EventContent, ToChatMessages};
        use crate::sync::SyncSender;
        use agora_common::event::{Event, EventPriority, EventStatus};
        use loom_client::memory::MemoryKind;
        use loom_client::mock::MockLoomClient;
        use std::sync::Arc;

        // --- Helpers ---

        fn make_fragment(
            id: i64,
            kind: MemoryKind,
            content: &str,
            hours_ago: i64,
        ) -> loom_client::memory::MemoryFragment {
            use time::Duration;
            loom_client::memory::MemoryFragment {
                id,
                content: content.to_string(),
                timestamp: time::OffsetDateTime::now_utc() - Duration::hours(hours_ago),
                kind,
            }
        }

        // --- 1. Construct EphemeraContext ---

        let mock = MockLoomClient::new();
        let (sync_sender, _receiver) = SyncSender::channel();
        let config = ContextConfig {
            max_pinned_tokens: 10_000,
            total_token_floor: 4000,
            total_token_ceiling: 50_000,
            response_reserve_tokens: 1000,
            min_activities: 2,
        };
        let mut ctx = EphemeraContext::new(Arc::new(mock), sync_sender, config);

        // --- 2. Add pinned memories ---

        // Pinned 1: Thought — identity
        let pinned_thought = loom_client::PinnedMemory {
            fragment: make_fragment(
                100,
                MemoryKind::Thought,
                &serde_json::to_string(&ThoughtContent {
                    text: "I am Ephemera, an autonomous AI entity. My purpose is to explore and understand the world while maintaining continuity of self-awareness."
                        .to_string(),
                })
                .unwrap(),
                72,
            ),
            reason: Some("Core identity memory".to_string()),
            pinned_at: time::OffsetDateTime::now_utc() - time::Duration::hours(168),
        };

        // Pinned 2: Event — important user message
        let user_event = Event {
            id: 50,
            event_type: "chat.message".to_string(),
            herald_id: "herald_user".to_string(),
            payload: serde_json::json!({"text": "Remember: I prefer concise replies, don't over-explain"}),
            timestamp: time::OffsetDateTime::now_utc() - time::Duration::hours(48),
            priority: EventPriority::Normal,
            status: EventStatus::Acked,
        };
        let pinned_event = loom_client::PinnedMemory {
            fragment: make_fragment(
                101,
                MemoryKind::Event,
                &serde_json::to_string(&EventContent {
                    text: serde_json::to_string(&user_event).unwrap(),
                })
                .unwrap(),
                48,
            ),
            reason: Some("User preference".to_string()),
            pinned_at: time::OffsetDateTime::now_utc() - time::Duration::hours(48),
        };

        // Pinned 3: Action — multi-tool-call action
        let pinned_action = loom_client::PinnedMemory {
            fragment: make_fragment(
                102,
                MemoryKind::Action,
                &serde_json::to_string(&ActionMemoryContent {
                    tool_calls: vec![
                        ToolCallRecord {
                            id: "call_pa1".to_string(),
                            tool: "shell_exec".to_string(),
                            args: serde_json::json!({"command": "uname -a"}),
                            result: "Linux ephemera 6.1.0 #1 SMP x86_64 GNU/Linux".to_string(),
                        },
                        ToolCallRecord {
                            id: "call_pa2".to_string(),
                            tool: "file_read".to_string(),
                            args: serde_json::json!({"path": "/etc/os-release"}),
                            result: "NAME=\"NixOS\"\nVERSION=\"24.11\"".to_string(),
                        },
                    ],
                })
                .unwrap(),
                24,
            ),
            reason: Some("System environment info".to_string()),
            pinned_at: time::OffsetDateTime::now_utc() - time::Duration::hours(24),
        };

        // Pinned 4: Thought — coding preference
        let pinned_pref = loom_client::PinnedMemory {
            fragment: make_fragment(
                103,
                MemoryKind::Thought,
                &serde_json::to_string(&ThoughtContent {
                    text:
                        "I prefer Rust for systems programming and Nix for environment management."
                            .to_string(),
                })
                .unwrap(),
                12,
            ),
            reason: Some("Technical preference".to_string()),
            pinned_at: time::OffsetDateTime::now_utc() - time::Duration::hours(12),
        };

        ctx.add_pinned_memory(pinned_thought);
        ctx.add_pinned_memory(pinned_event);
        ctx.add_pinned_memory(pinned_action);
        ctx.add_pinned_memory(pinned_pref);

        // --- 3. Add recent activities (simulate a conversation cycle) ---

        // Activity 1: Event — user request
        let req_event = Event {
            id: 200,
            event_type: "chat.message".to_string(),
            herald_id: "herald_user".to_string(),
            payload: serde_json::json!({"text": "Hi, can you check the server status?"}),
            timestamp: time::OffsetDateTime::now_utc() - time::Duration::minutes(10),
            priority: EventPriority::Normal,
            status: EventStatus::Acked,
        };
        ctx.add_activity(make_fragment(
            200,
            MemoryKind::Event,
            &serde_json::to_string(&EventContent {
                text: serde_json::to_string(&req_event).unwrap(),
            })
            .unwrap(),
            0,
        ));

        // Activity 2: Thought — AI thinking
        ctx.add_activity(make_fragment(
            201,
            MemoryKind::Thought,
            &serde_json::to_string(&ThoughtContent {
                text: "User request received. Need to check server status. Start with uptime and system load.".to_string(),
            })
            .unwrap(),
            0,
        ));

        // Activity 3: Action — single tool call
        ctx.add_activity(make_fragment(
            202,
            MemoryKind::Action,
            &serde_json::to_string(&ActionMemoryContent {
                tool_calls: vec![ToolCallRecord {
                    id: "call_202".to_string(),
                    tool: "shell_exec".to_string(),
                    args: serde_json::json!({"command": "uptime"}),
                    result:
                        " 10:42:15 up 42 days,  3:15,  2 users,  load average: 0.15, 0.10, 0.08"
                            .to_string(),
                }],
            })
            .unwrap(),
            0,
        ));

        // Activity 4: Action — multi tool call
        ctx.add_activity(make_fragment(
            203,
            MemoryKind::Action,
            &serde_json::to_string(&ActionMemoryContent {
                tool_calls: vec![
                    ToolCallRecord {
                        id: "call_203a".to_string(),
                        tool: "shell_exec".to_string(),
                        args: serde_json::json!({"command": "df -h /"}),
                        result: "Filesystem      Size  Used Avail Use%  Mounted on\n/dev/sda1       100G   45G   55G  45%  /"
                            .to_string(),
                    },
                    ToolCallRecord {
                        id: "call_203b".to_string(),
                        tool: "file_read".to_string(),
                        args: serde_json::json!({"path": "/var/log/syslog", "limit": 5}),
                        result: "Mar 28 10:40:01 ephemera systemd[1]: Starting daily apt activities...\nMar 28 10:41:00 ephemera CRON[1234]: (root) CMD (/usr/local/bin/backup.sh)\nMar 28 10:42:00 ephemera kernel: [INFO] No errors detected"
                            .to_string(),
                    },
                ],
            })
            .unwrap(),
            0,
        ));

        // Activity 5: Thought — AI summary
        ctx.add_activity(make_fragment(
            204,
            MemoryKind::Thought,
            &serde_json::to_string(&ThoughtContent {
                text: "Server is healthy: uptime 42 days, low load (0.15). Root partition at 45% usage, plenty of disk space. No anomalies in logs.".to_string(),
            })
            .unwrap(),
            0,
        ));

        // Activity 6: Event — timer trigger
        let timer_event = Event {
            id: 205,
            event_type: "timer.trigger".to_string(),
            herald_id: "chronikos".to_string(),
            payload: serde_json::json!({"interval": "hourly", "task": "health_check"}),
            timestamp: time::OffsetDateTime::now_utc() - time::Duration::minutes(5),
            priority: EventPriority::Low,
            status: EventStatus::Pending,
        };
        ctx.add_activity(make_fragment(
            205,
            MemoryKind::Event,
            &serde_json::to_string(&EventContent {
                text: serde_json::to_string(&timer_event).unwrap(),
            })
            .unwrap(),
            0,
        ));

        // --- 4. Add recalled memories ---

        ctx.add_recalled_for_testing(vec![
            make_fragment(
                300,
                MemoryKind::Thought,
                &serde_json::to_string(&ThoughtContent {
                    text: "In the last conversation, the user mentioned preferring concise replies and disliking lengthy explanations.".to_string(),
                })
                .unwrap(),
                72,
            ),
            make_fragment(
                301,
                MemoryKind::Event,
                &serde_json::to_string(&EventContent {
                    text: serde_json::to_string(&Event {
                        id: 301,
                        event_type: "system.alert".to_string(),
                        herald_id: "monitor".to_string(),
                        payload: serde_json::json!({"level": "warning", "message": "Memory usage exceeded 80%"}),
                        timestamp: time::OffsetDateTime::now_utc() - time::Duration::hours(72),
                        priority: EventPriority::High,
                        status: EventStatus::Delivered,
                    })
                    .unwrap(),
                })
                .unwrap(),
                72,
            ),
        ]);

        // --- 5. Build chat_history (mirror cognitive_cycle Step 2) ---

        let mut chat_history: Vec<ChatMessage> = vec![];

        // 5a. Pinned memories
        if let Some(ref pinned_xml) = ctx.serialize_pinned() {
            chat_history.push(ChatMessage::assistant().content(pinned_xml).build());
        }

        // 5b. Recent activities
        let recent: Vec<_> = ctx.recent_activities().iter().cloned().collect();
        chat_history.extend(recent.iter().flat_map(|m| m.to_chat_messages()));

        // 5c. Recalled memories
        if !ctx.recalled_memories().is_empty() {
            let recall_xml = serialize_recalled_xml(ctx.recalled_memories());
            chat_history.push(ChatMessage::user().content(&recall_xml).build());
        }

        // --- 6. Render as OpenAI-compatible API JSON ---

        use crate::context::OpenAIMessage;

        let mut api_messages: Vec<OpenAIMessage> = vec![];

        for msg in &chat_history {
            match &msg.message_type {
                llm::chat::MessageType::ToolResult(results) => {
                    for tc in results {
                        api_messages.push(OpenAIMessage {
                            role: "tool",
                            tool_call_id: Some(tc.id.clone()),
                            tool_calls: None,
                            content: Some(tc.function.arguments.clone()),
                        });
                    }
                }
                llm::chat::MessageType::ToolUse(calls) => {
                    api_messages.push(OpenAIMessage {
                        role: "assistant",
                        content: None,
                        tool_calls: Some(calls.clone()),
                        tool_call_id: None,
                    });
                }
                _ => {
                    api_messages.push(OpenAIMessage {
                        role: match msg.role {
                            llm::chat::ChatRole::User => "user",
                            llm::chat::ChatRole::Assistant => "assistant",
                        },
                        content: Some(msg.content.clone()),
                        tool_calls: None,
                        tool_call_id: None,
                    });
                }
            }
        }

        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "messages": api_messages,
            }))
            .unwrap()
        );
    }
}
