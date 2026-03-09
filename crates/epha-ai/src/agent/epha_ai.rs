use crate::agent::{CommonPrompt, State};
use crate::context::EphemeraContext;
use crate::sync::{start_sync_task, SyncSender};
use crate::tools::{MemoryGet, MemoryPin, MemoryRecent, MemoryTimeline, MemoryUnpin, StateTransition};
use agora_client::AgoraClient;
use epha_agent::context::Context;
use epha_agent::tools::{file_system_tool_set, shell_tool_set, shell::TmuxBackend};
use loom_client::memory::{MemoryFragment, MemoryKind};
use loom_client::LoomClient;
use rig::{
    agent::Agent,
    client::CompletionClient,
    completion::{AssistantContent, Completion, Message},
    message::{ToolResult, ToolResultContent, UserContent},
    providers::deepseek::{Client, CompletionModel},
    tool::{ToolSet, server::ToolServer},
    OneOrMany,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{info, warn};
use uuid::Uuid;

pub struct EphemeraAI {
    state: Arc<Mutex<State>>,
    agent: Agent<CompletionModel>,
    context: Context<EphemeraContext>,
    agora_client: Arc<AgoraClient>,
    config: crate::config::Config,
    sync_sender: SyncSender,
}

impl EphemeraAI {
    pub async fn new(
        config: crate::config::Config,
        loom_client: Arc<LoomClient>,
        completion_client: Client,
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
            config.context.max_pinned_count,
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
        let backend = TmuxBackend::new(&session_name).await
            .map_err(|e| anyhow::anyhow!("Failed to create tmux backend '{}': {}", session_name, e))?;

        // 5. Initialize Agora client with health check
        let agora_client = Arc::new(AgoraClient::new(&config.agora.url, http_client));
        info!("Initializing Agora client: {}", config.agora.url);
        agora_client.health_check().await
            .map_err(|e| anyhow::anyhow!("Agora service unavailable at '{}': {}", config.agora.url, e))?;
        info!("Agora service is available");

        // 6. Create tool server with static tools
        let tool_server = ToolServer::new()
            .tool(MemoryGet::new(loom_client.clone(), context_data.clone()))
            .tool(MemoryRecent::new(loom_client.clone(), context_data.clone()))
            .tool(MemoryTimeline::new(loom_client.clone(), context_data.clone()))
            .tool(MemoryPin::new(loom_client.clone(), context_data.clone()))
            .tool(MemoryUnpin::new(loom_client.clone(), context_data.clone()))
            .tool(StateTransition::new(state.clone()));

        let tool_server_handle = tool_server.run();

        // 7. Create ToolSet with boxed tools and append to server
        let mut boxed_toolset = ToolSet::default();

        // Add file system tools
        for tool in file_system_tool_set() {
            boxed_toolset.add_tool_boxed(tool);
        }

        // Add shell tools
        for tool in shell_tool_set(Arc::new(tokio::sync::Mutex::new(backend))) {
            boxed_toolset.add_tool_boxed(tool);
        }

        // Append the boxed toolset to the running server
        tool_server_handle.append_toolset(boxed_toolset).await?;

        // 8. Build agent with tool server handle
        let agent = completion_client
            .agent(&config.llm.model)
            .preamble(&common_prompt.content)
            .tool_server_handle(tool_server_handle)
            .build();

        Ok(Self {
            state,
            agent,
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
                    tokio::time::sleep(Duration::from_millis(self.config.dormant_tick_interval_ms)).await;
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
        let events = self.agora_client
            .fetch_events(None)
            .await?;

        if !events.is_empty() {
            // Collect event IDs for acknowledgment
            let event_ids: Vec<u64> = events.iter().map(|e| e.id).collect();

            // Add events to context
            self.context.data()
                .lock()
                .await
                .add_agora_events(events);

            // Acknowledge processed events
            self.agora_client.ack_events(event_ids).await?;
        }

        // 2. Prepare context (including newly added events)
        let context_str = self.context.serialize().await;

        // 3. Explicit multi-turn loop
        let initial_prompt = format!("Current Context:\n{}", context_str);
        let mut chat_history: Vec<Message> = vec![];
        let mut current_prompt = initial_prompt;
        let mut current_depth = 0;

        loop {
            // 3.1 Get completion
            let builder = self.agent
                .completion(&current_prompt, chat_history.clone())
                .await?;

            let response = builder.send().await?;

            // 3.2 Extract and save Thought (AI text response)
            let texts: Vec<String> = response.choice
                .iter()
                .filter_map(|c| match c {
                    AssistantContent::Text(t) => Some(t.text.clone()),
                    _ => None,
                })
                .collect();

            if !texts.is_empty() {
                let thought = texts.join("\n");
                self.save_thought(&thought);
            }

            // 3.3 Extract tool calls
            let tool_calls: Vec<_> = response.choice
                .iter()
                .filter_map(|c| match c {
                    AssistantContent::ToolCall(tc) => Some(tc.clone()),
                    _ => None,
                })
                .collect();

            // 3.4 If no tool calls, we're done
            if tool_calls.is_empty() {
                break;
            }

            // 3.5 Check depth limit
            current_depth += 1;
            if current_depth > self.config.llm.max_turns {
                warn!("Max depth {} reached, continuing next cycle", current_depth);
                break;
            }

            // 3.6 Execute tools and build results
            let mut tool_results: Vec<ToolResult> = vec![];

            for tc in &tool_calls {
                let args_str = tc.function.arguments.to_string();
                let result = self.agent.tool_server_handle
                    .call_tool(&tc.function.name, &args_str)
                    .await?;

                // Save Action memory
                self.save_action(&tc.function.name, &args_str, &result);

                // Build tool result for chat history
                tool_results.push(ToolResult {
                    id: tc.id.clone(),
                    call_id: tc.call_id.clone(),
                    content: OneOrMany::one(ToolResultContent::text(result)),
                });
            }

            // 3.7 Update chat history for next iteration
            chat_history.push(Message::Assistant {
                id: None,
                content: response.choice,
            });
            let user_contents: Vec<UserContent> = tool_results
                .into_iter()
                .map(UserContent::ToolResult)
                .collect();
            chat_history.push(Message::User {
                content: OneOrMany::many(user_contents)
                    .expect("tool_results should have at least one item"),
            });

            // 3.8 Clear prompt for subsequent iterations
            current_prompt = String::new();
        }

        Ok(())
    }

    /// Save a Thought memory (AI's text response)
    fn save_thought(&self, text: &str) {
        let fragment = MemoryFragment {
            id: 0,
            content: text.to_string(),
            timestamp: OffsetDateTime::now_utc(),
            kind: MemoryKind::Thought,
        };
        self.sync_sender.send(fragment);
    }

    /// Save an Action memory (tool call)
    fn save_action(&self, tool: &str, args: &str, result: &str) {
        let content = serde_json::json!({
            "tool": tool,
            "args": serde_json::from_str(args).unwrap_or(serde_json::json!(args)),
            "result": result,
            "status": "success"
        });
        let fragment = MemoryFragment {
            id: 0,
            content: content.to_string(),
            timestamp: OffsetDateTime::now_utc(),
            kind: MemoryKind::Action,
        };
        self.sync_sender.send(fragment);
    }
}
