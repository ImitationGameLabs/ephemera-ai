use crate::tools::AgentTool;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agent::State;

/// Arguments for state transition
#[derive(Debug, Deserialize)]
pub struct StateTransitionArgs {
    /// The target existence state to transition to
    pub mode: State,
    /// Reason for the transition
    pub reason: String,
}

/// Tool for transitioning between existence states
pub struct StateTransition {
    state: Arc<Mutex<State>>,
}

impl StateTransition {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl AgentTool for StateTransition {
    fn name(&self) -> &str {
        "state_transition"
    }

    fn description(&self) -> &str {
        "Transition between existence states: Active (normal mode), Dormant (slow mode), or Suspended (exit loop)"
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "enum": ["Active", "Dormant", "Suspended"],
                    "description": "The target existence state"
                },
                "reason": {
                    "type": "string",
                    "description": "Reason for making this state transition"
                }
            },
            "required": ["mode", "reason"]
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: StateTransitionArgs = serde_json::from_str(args_json)?;
        let mut state = self.state.lock().await;

        let current = *state;
        *state = args.mode;

        Ok(format!(
            "State changed from {:?} to {:?}. Reason: {}",
            current, args.mode, args.reason
        ))
    }
}
