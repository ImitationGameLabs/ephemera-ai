use epha_agent::state_machine::StateMachine;
use std::sync::{Arc, Mutex};
use rig::{
    completion::Completion,
    message::AssistantContent,
};

/// Executor for state machine that handles the execution logic
pub struct StateMachineExecutor {
    state_machine: Arc<Mutex<StateMachine>>,
}

impl StateMachineExecutor {
    /// Create a new StateMachineExecutor
    pub fn new(state_machine: Arc<Mutex<StateMachine>>) -> Self {
        Self { state_machine }
    }

    /// Execute the current state with given context
    pub async fn execute_current_state(&self, context: &str) -> anyhow::Result<String> {
        let agent = {
            let sm = self.state_machine.lock().unwrap();
            let current_state = sm.current_state()?;

            current_state.completion_agent.as_ref()
                .ok_or_else(|| anyhow::anyhow!("Current state '{}' has no completion agent configured",
                                               current_state.prompt_data.name))?
                .clone()
        };

        // Build the full prompt with context
        let full_prompt = {
            let sm = self.state_machine.lock().unwrap();
            let current_state = sm.current_state()?;

            format!("{}\n\nCurrent Context:\n{}",
                   current_state.prompt_data.prompt,
                   context)
        };

        // Execute the agent (rig will handle multi-turn and tool calls automatically)
        let completion_result = agent
            .completion(full_prompt, vec![])
            .await?;

        let result = completion_result.send().await?;

        // Extract text content from the response
        match result.choice.first() {
            AssistantContent::Text(text) => Ok(text.text().to_string()),
            _ => Err(anyhow::anyhow!("Invalid assistant content type")),
        }
    }
}