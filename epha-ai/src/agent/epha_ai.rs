use epha_agent::state_machine::{StateMachine, State, load_state_prompts_from_directory};
use epha_agent::context::Context;
use rig::{
    agent::Agent,
    providers::deepseek::Client,
    client::CompletionClient,
    providers::deepseek::CompletionModel,
};
use time::OffsetDateTime;
use std::sync::{Arc, Mutex};
use epha_memory::{HybridMemoryManager, MemoryFragment, SubjectiveMetadata, ObjectiveMetadata, MemorySource};
use crate::agent::{EphemeraContext, CommonPrompt, StateMachineExecutor, MemoryFragmentBuilder};
use crate::tools::{GetMessages, MemoryRecall, SelectMemories, MemoryCache, SendMessage, StateTransition};

pub struct EphemeraAI {
    state_machine: Arc<Mutex<StateMachine>>,
    completion_client: Arc<Client>,
    context: Context<EphemeraContext>,
    memory_manager: Arc<HybridMemoryManager>,
    common_prompt: CommonPrompt,
    executor: StateMachineExecutor,
    memory_cache: Arc<MemoryCache>,
}

impl EphemeraAI {
    pub fn new(
        completion_client: Client,
        memory_manager: Arc<HybridMemoryManager>,
        model: &str
    ) -> Self {
        // Load common prompt
        let common_prompt = CommonPrompt::from_file("prompts/common.md")
            .expect("Failed to load common prompt");

        // Stage 1: Load state prompts and create states without agents
        let state_prompts = load_state_prompts_from_directory("prompts/states")
            .expect("Failed to load state prompts from directory");

        let states: Vec<State> = state_prompts.into_iter()
            .map(|prompt| State::new(prompt))
            .collect();

        // Stage 2: Create StateMachine with states (no agents yet)
        let state_machine = StateMachine::new(states, "reasoning")
            .map(|sm| Arc::new(Mutex::new(sm)))
            .expect("Failed to create StateMachine");

        // Stage 2.5: Create shared memory cache
        let memory_cache = Arc::new(MemoryCache::new());

        // Create shared context first
        let context_data = Arc::new(Mutex::new(EphemeraContext::new()));

        // Stage 3: Create agents and assign them to states
        init_agents(&completion_client, model, memory_manager.clone(), &state_machine, &common_prompt, &memory_cache, &context_data)
            .expect("Failed to initialize agents");

        let executor = StateMachineExecutor::new(state_machine.clone());

        Self {
            state_machine,
            completion_client: Arc::new(completion_client),
            context: Context::new(context_data),
            memory_manager,
            common_prompt,
            executor,
            memory_cache,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            self.execute_current_round().await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    async fn execute_current_round(&mut self) -> anyhow::Result<()> {
        // Prepare context for current state
        let context_str = self.context.serialize();

        // Execute current state through StateMachineExecutor
        let result = self.executor.execute_current_state(&context_str).await?;

        // Update context with execution result
        self.update_context(result).await?;

        Ok(())
    }
    
    async fn update_context(&mut self, result: String) -> anyhow::Result<()> {
        // Application-specific context update logic
        let activity_fragment = MemoryFragmentBuilder::action("state_execution".to_string(), result).build();
        self.context.data().lock().unwrap().add_activity(activity_fragment);
        Ok(())
    }

    // Memory-related methods
    pub async fn save_response_memory(
        &mut self,
        response: String,
        additional_tags: Vec<String>
    ) -> anyhow::Result<()> {
        let response_timestamp = OffsetDateTime::now_utc();

        let mut tags = vec!["response".to_string()];
        tags.extend(additional_tags);

        let response_memory = MemoryFragment {
            id: 0, // Will be set by database after insertion
            content: response.clone(),
            subjective_metadata: SubjectiveMetadata {
                importance: 60,
                confidence: 90,
                tags,
                notes: String::new(),
            },
            objective_metadata: ObjectiveMetadata {
                created_at: response_timestamp.unix_timestamp(),
                source: MemorySource::dialogue_response(),
            },
            associations: vec![],
        };

        // Note: This requires mutable access to memory_manager
        // For now, we'll need to handle this differently or use interior mutability
        // TODO: Implement proper mutable access pattern
        println!("Would save memory: {}", response_memory.content);

        // Update context
        let activity_fragment = MemoryFragmentBuilder::action(
            "memory_save".to_string(),
            format!("Saved response memory with {} characters", response.len())
        ).build();
        self.context.data().lock().unwrap().add_activity(activity_fragment);

        Ok(())
    }
}

/// Initialize agents for all states in the state machine
fn init_agents(
    completion_client: &Client,
    model: &str,
    memory_manager: Arc<HybridMemoryManager>,
    state_machine: &Arc<Mutex<StateMachine>>,
    common_prompt: &CommonPrompt,
    memory_cache: &Arc<MemoryCache>,
    context_data: &Arc<Mutex<EphemeraContext>>,
) -> anyhow::Result<()> {
    let state_names: Vec<String> = state_machine.lock().unwrap().get_state_names();

    // Create agents for each state
    for state_name in state_names {
        // Get state reference
        let mut sm = state_machine.lock().unwrap();
        let state = sm.get_state(&state_name)
            .ok_or_else(|| anyhow::anyhow!("State '{}' not found", state_name))?;
    
        let agent = create_agent_for_state(
            completion_client,
            model,
            &memory_manager,
            state_machine,
            &state,
            common_prompt,
            memory_cache,
            context_data
        )?;

        if let Some(state) = sm.get_state_mut(&state_name) {
            state.with_agent(agent);
        } 
    }

    Ok(())
}

/// Create an agent for a specific state
fn create_agent_for_state(
    completion_client: &Client,
    model: &str,
    memory_manager: &Arc<HybridMemoryManager>,
    state_machine: &Arc<Mutex<StateMachine>>,
    state: &State,
    common_prompt: &CommonPrompt,
    memory_cache: &Arc<MemoryCache>,
    context_data: &Arc<Mutex<EphemeraContext>>,
) -> anyhow::Result<Agent<CompletionModel>> {
    // Get state prompt for combined prompt creation
    let prompt = &state.prompt_data;

    let combined_prompt = common_prompt.combine_with_state_prompt(&prompt.prompt);

    // Build agent with appropriate tools based on state name
    let agent_builder = completion_client
        .agent(model)
        .preamble(&combined_prompt);

    let agent = match prompt.name.as_str() {
        "perception" => agent_builder.tool(GetMessages).build(),
        "recall" => {
            agent_builder
                .tool(MemoryRecall::new(memory_manager.clone(), memory_cache.clone()))
                .tool(SelectMemories::new(memory_cache.clone(), context_data.clone()))
                .build()
        },
        "reasoning" => agent_builder.tool(StateTransition::new(state_machine.clone())).build(),
        "output" => agent_builder.tool(SendMessage).build(),
        _ => agent_builder.build(),
    };

    Ok(agent)
}
