use crate::agent::{CommonPrompt, State};
use crate::context::EphemeraContext;
use crate::context::memory_constructors::from_action;
use crate::tools::{GetMessages, MemoryGet, MemoryRecent, MemoryTimeline, SendMessage, StateTransition};
use atrium_client::AuthenticatedClient;
use epha_agent::context::Context;
use loom_client::LoomClient;
use rig::{agent::Agent, client::CompletionClient, completion::Prompt, providers::deepseek::Client, providers::deepseek::CompletionModel};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::info;

pub struct EphemeraAI {
    state: Arc<Mutex<State>>,
    agent: Agent<CompletionModel>,
    context: Context<EphemeraContext>,
    config: crate::config::Config,
}

impl EphemeraAI {
    pub fn new(
        completion_client: Client,
        loom_client: Arc<LoomClient>,
        dialogue_client: Arc<AuthenticatedClient>,
        config: crate::config::Config,
    ) -> Self {
        // 1. Create shared state
        let state = Arc::new(Mutex::new(State::default()));

        // 2. Load common prompt
        let common_prompt =
            CommonPrompt::from_file("prompts/common.md").expect("Failed to load common prompt");

        // 3. Create context
        let context_data = Arc::new(Mutex::new(EphemeraContext::new(loom_client.clone())));

        // 4. Create single Agent with all tools
        let agent = completion_client
            .agent(&config.llm.model)
            .preamble(&common_prompt.content)
            .tool(GetMessages::new(dialogue_client.clone()))
            .tool(SendMessage::new(dialogue_client.clone()))
            .tool(MemoryGet::new(loom_client.clone(), context_data.clone()))
            .tool(MemoryRecent::new(loom_client.clone(), context_data.clone()))
            .tool(MemoryTimeline::new(loom_client.clone(), context_data.clone()))
            .tool(StateTransition::new(state.clone()))
            .build();

        Self {
            state,
            agent,
            context: Context::new(context_data),
            config,
        }
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
        // Prepare context
        let context_str = self.context.serialize();

        // Execute agent (rig handles tool calls including StateTransition internally)
        let prompt = format!("Current Context:\n{}", context_str);
        let result = self.agent.prompt(&prompt).await?;

        // Update context with result
        self.update_context(result).await?;

        Ok(())
    }

    async fn update_context(&mut self, result: String) -> anyhow::Result<()> {
        let fragment = from_action(format!("cognitive_cycle: {}", result), "cycle").build();
        self.context.data().lock().unwrap().add_activity(fragment);
        Ok(())
    }
}
