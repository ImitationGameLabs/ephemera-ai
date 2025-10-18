use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use epha_agent::state_machine::StateMachine;
use std::sync::{Arc, Weak, Mutex};
use std::fmt;

/// Arguments for state transition
#[derive(Debug, Deserialize)]
pub struct StateTransitionArgs {
    /// The target state to transition to
    pub target_state: String,
    /// Reason for the transition
    pub reason: String,
}

/// Error type for state transition tool
#[derive(Debug)]
pub struct StateTransitionError(String);

impl fmt::Display for StateTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "State transition error: {}", self.0)
    }
}

impl std::error::Error for StateTransitionError {}


/// Tool for transitioning between states
pub struct StateTransition {
    state_machine: Weak<Mutex<StateMachine>>,
}

impl StateTransition {
    pub fn new(state_machine: Arc<Mutex<StateMachine>>) -> Self {
        Self {
            state_machine: Arc::downgrade(&state_machine)
        }
    }
}

impl Tool for StateTransition {
    const NAME: &'static str = "state_transition";
    type Error = StateTransitionError;
    type Args = StateTransitionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(serde_json::json!({
            "name": "state_transition",
            "description": "Transition to a different state in the reasoning cycle",
            "parameters": {
                "type": "object",
                "properties": {
                    "target_state": {
                        "type": "string",
                        "description": "The target state to transition to (e.g., 'perception', 'recall', 'reasoning', 'output')"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Reason for making this state transition"
                    }
                },
                "required": ["target_state", "reason"]
            }
        })).expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Try to upgrade the weak reference to Arc
        let state_machine_arc = self.state_machine.upgrade().ok_or_else(|| {
            StateTransitionError("State machine has been dropped".to_string())
        })?;

        let mut state_machine = state_machine_arc.lock().map_err(|e| {
            StateTransitionError(format!("Failed to acquire state machine lock: {}", e))
        })?;

        // Get current state before transition
        let current_state_name = state_machine.current_state_name().to_string();

        // Validate target state exists
        if state_machine.get_state(&args.target_state).is_none() {
            return Ok(format!("Error: Target state '{}' does not exist", args.target_state));
        }

        // Check if already in target state
        if current_state_name == args.target_state {
            return Ok(format!("Info: Already in state '{}'", args.target_state));
        }

        // Perform the transition
        match state_machine.transition_to(args.target_state.clone(), args.reason.clone()) {
            Ok(round_count) => Ok(format!(
                "Successfully transitioned from '{}' to '{}' with reason: '{}'. Previous round count: {}",
                current_state_name, args.target_state, args.reason, round_count
            )),
            Err(e) => Ok(format!("Failed to transition: {}", e)),
        }
    }
}