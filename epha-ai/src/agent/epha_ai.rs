use crate::agent::{CommonPrompt, State};
use crate::context::EphemeraContext;
use crate::context::memory_constructors::from_action;
use crate::tools::{GetMessages, MemoryGet, MemoryRecent, MemoryTimeline, SendMessage, StateTransition};
use atrium_client::AuthenticatedClient;
use epha_agent::context::Context;
use epha_agent::subscriber::{SubscriptionManager, SubscriberConfig};
use epha_agent::tools::{file_system_tool_set, shell_tool_set, subscription_tool_set, shell::TmuxBackend};
use loom_client::LoomClient;
use rig::{
    agent::Agent,
    client::CompletionClient,
    completion::Prompt,
    providers::deepseek::{Client, CompletionModel},
    tool::{ToolSet, server::ToolServer},
};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
use uuid::Uuid;

pub struct EphemeraAI {
    state: Arc<std::sync::Mutex<State>>,
    agent: Agent<CompletionModel>,
    context: Context<EphemeraContext>,
    subscription_manager: Arc<tokio::sync::Mutex<SubscriptionManager>>,
    config: crate::config::Config,
}

impl EphemeraAI {
    pub async fn new(
        config: crate::config::Config,
        dialogue_client: Arc<AuthenticatedClient>,
        loom_client: Arc<LoomClient>,
        completion_client: Client,
    ) -> anyhow::Result<Self> {
        // 1. Create shared state
        let state = Arc::new(std::sync::Mutex::new(State::default()));

        // 2. Load common prompt
        let common_prompt = CommonPrompt::from_file("prompts/common.md")?;

        // 3. Create context
        let context_data = Arc::new(std::sync::Mutex::new(EphemeraContext::new(loom_client.clone())));

        // 4. Initialize shell backend
        let session_name = format!("ephemera-ai-{}", Uuid::new_v4().simple());
        info!("Creating tmux session: {}", session_name);
        let backend = TmuxBackend::new(&session_name).await
            .map_err(|e| anyhow::anyhow!("Failed to create tmux backend '{}': {}", session_name, e))?;

        // 5. Create SubscriptionManager
        let subscriber_config = SubscriberConfig {
            heartbeat_interval_seconds: config.subscriber.heartbeat_interval_seconds,
            degraded_timeout_seconds: config.subscriber.degraded_timeout_seconds,
            disconnect_timeout_seconds: config.subscriber.disconnect_timeout_seconds,
        };
        let subscription_manager = Arc::new(tokio::sync::Mutex::new(
            SubscriptionManager::new(subscriber_config)
        ));

        // 6. Create tool server with static tools
        let tool_server = ToolServer::new()
            .tool(GetMessages::new(dialogue_client.clone()))
            .tool(SendMessage::new(dialogue_client.clone()))
            .tool(MemoryGet::new(loom_client.clone(), context_data.clone()))
            .tool(MemoryRecent::new(loom_client.clone(), context_data.clone()))
            .tool(MemoryTimeline::new(loom_client.clone(), context_data.clone()))
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

        // Add subscription management tools
        for tool in subscription_tool_set(subscription_manager.clone()) {
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
            subscription_manager,
            config,
        })
    }

    pub async fn live(&mut self) -> anyhow::Result<()> {
        loop {
            let state = *self.state.lock().unwrap();
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
        // 1. Process pending Producer events first
        let pending_events = self.subscription_manager
            .lock()
            .await
            .drain_pending_messages();

        if !pending_events.is_empty() {
            self.context.data()
                .lock()
                .unwrap()
                .add_producer_events(pending_events);
        }

        // 2. Prepare context (including newly added events)
        let context_str = self.context.serialize();

        // 3. Execute agent (rig handles tool calls including StateTransition internally)
        let prompt = format!("Current Context:\n{}", context_str);
        let result = self.agent.prompt(&prompt).await?;

        // 4. Update context with result
        self.update_context(result).await?;

        Ok(())
    }

    async fn update_context(&mut self, result: String) -> anyhow::Result<()> {
        let fragment = from_action(format!("cognitive_cycle: {}", result), "cycle").build();
        self.context.data().lock().unwrap().add_activity(fragment);
        Ok(())
    }
}
