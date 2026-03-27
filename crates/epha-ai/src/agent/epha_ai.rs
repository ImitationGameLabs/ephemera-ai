use crate::agent::{CommonPrompt, State};
use crate::context::EphemeraContext;
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
use time::OffsetDateTime;
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

        // 2. Prepare context (including newly added events)
        let context_str = self.context.serialize().await;

        // 3. Explicit multi-turn loop
        let initial_prompt = format!("Current Context:\n{}", context_str);
        let mut chat_history: Vec<ChatMessage> = vec![];
        let mut current_prompt = initial_prompt;
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
            let mut tool_results: Vec<(String, String, String)> = vec![];

            for tc in &tool_calls {
                let tool_name = &tc.function.name;
                let args_str = &tc.function.arguments;

                let result = self
                    .tool_dispatch
                    .call_tool(tool_name, args_str)
                    .await
                    .map_err(|e| anyhow::anyhow!("{}", e))?;

                // Save Action memory
                self.save_action(tool_name, args_str, &result);

                tool_results.push((tc.id.clone(), tool_name.clone(), result));
            }

            // 3.7 Update chat history for next iteration
            // Add assistant message with tool calls
            chat_history
                .push(ChatMessage::assistant().tool_use(tool_calls.clone()).content("").build());

            // Add tool result message
            // The llm crate's OpenAI compatible provider expands ToolResult into
            // separate "tool" role messages using ToolCall.id as tool_call_id
            // and ToolCall.function.arguments as the content.
            let result_tool_calls: Vec<ToolCall> = tool_results
                .into_iter()
                .map(|(id, tool_name, result)| ToolCall {
                    id,
                    call_type: "function".to_string(),
                    function: FunctionCall { name: tool_name, arguments: result },
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
