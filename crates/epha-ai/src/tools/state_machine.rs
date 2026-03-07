use crate::agent::State;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Arguments for state transition
#[derive(Debug, Deserialize)]
pub struct StateTransitionArgs {
    /// The target life state to transition to
    pub mode: State,
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

/// Tool for transitioning between life states
pub struct StateTransition {
    state: Arc<Mutex<State>>,
}

impl StateTransition {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
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
            "description": "Transition between life states: Active (normal mode), Dormant (slow mode), or Suspended (exit loop)",
            "parameters": {
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "enum": ["Active", "Dormant", "Suspended"],
                        "description": "The target life state"
                    },
                    "reason": {
                        "type": "string",
                        "description": "Reason for making this state transition"
                    }
                },
                "required": ["mode", "reason"]
            }
        })).expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut state = self.state.lock().map_err(|e| {
            StateTransitionError(format!("Failed to acquire state lock: {}", e))
        })?;

        let current = *state;
        *state = args.mode;

        Ok(format!(
            "State changed from {:?} to {:?}. Reason: {}",
            current, args.mode, args.reason
        ))
    }
}
