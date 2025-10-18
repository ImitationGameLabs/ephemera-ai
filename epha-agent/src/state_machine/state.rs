use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use rig::{
    agent::Agent,
    providers::deepseek::CompletionModel,
};

/// Represents a state in the agent's reasoning cycle
#[derive(Clone, Serialize, Deserialize)]
pub struct State {
    pub prompt_data: super::state_loader::StatePrompt,  // State data from prompts
    pub min_round_interval: Option<u32>, // Minimum round interval
    pub max_round_interval: Option<u32>, // Maximum round interval

    // Runtime execution agent (not serialized)
    #[serde(skip)]
    pub completion_agent: Option<Agent<CompletionModel>>,
}

impl State {
    /// Create a new state with prompt data only
    pub fn new(prompt_data: super::state_loader::StatePrompt) -> Self {
        Self {
            prompt_data,
            min_round_interval: None,
            max_round_interval: None,
            completion_agent: None,
        }
    }

    /// Set round frequency constraints
    pub fn with_round_constraints(mut self, min_interval: Option<u32>, max_interval: Option<u32>) -> Self {
        self.min_round_interval = min_interval;
        self.max_round_interval = max_interval;
        self
    }

    /// Set the completion agent for this state (builder pattern)
    pub fn with_completion_agent(mut self, agent: Agent<CompletionModel>) -> Self {
        self.completion_agent = Some(agent);
        self
    }

    /// Set the completion agent for this state (mutable reference)
    pub fn with_agent(&mut self, agent: Agent<CompletionModel>) {
        self.completion_agent = Some(agent);
    }

}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("prompt_data", &self.prompt_data)
            .field("min_round_interval", &self.min_round_interval)
            .field("max_round_interval", &self.max_round_interval)
            .field("completion_agent", &"<agent>")
            .finish()
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "State('{}': {}) - {}",
               self.prompt_data.name, self.prompt_data.description,
               if let Some(max) = self.max_round_interval {
                   format!("Max interval: {} rounds", max)
               } else {
                   "No interval constraint".to_string()
               })
    }
}

/// Represents state transition information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from_state: String,
    pub to_state: String,
    pub round_count: u32,
    pub reason: String,
}

/// Tracks round counts for each state
#[derive(Debug, Clone, Default)]
pub struct StateRoundTracker {
    current_round: u32,
    current_state: String,
    state_round_counts: HashMap<String, u32>,
    global_round_count: u32,
    transition_history: Vec<StateTransition>,
}

impl StateRoundTracker {
    pub fn new(initial_state: String) -> Self {
        let mut tracker = Self {
            current_round: 0,
            current_state: initial_state.clone(),
            state_round_counts: HashMap::new(),
            global_round_count: 0,
            transition_history: Vec::new(),
        };
        tracker.state_round_counts.insert(initial_state, 0);
        tracker
    }

    /// Increment round count for current state
    pub fn increment_round(&mut self) {
        self.current_round += 1;
        self.global_round_count += 1;

        *self.state_round_counts.entry(self.current_state.clone()).or_insert(0) += 1;
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: String, reason: String) -> u32 {
        let previous_round_count = self.current_round;

        let transition = StateTransition {
            from_state: self.current_state.clone(),
            to_state: new_state.clone(),
            round_count: previous_round_count,
            reason,
        };

        self.transition_history.push(transition);

        self.current_state = new_state;
        self.current_round = 0;

        // Initialize count for new state if needed
        self.state_round_counts.entry(self.current_state.clone()).or_insert(0);

        previous_round_count
    }

    /// Get current state
    pub fn current_state(&self) -> &str {
        &self.current_state
    }

    /// Get round count in current state
    pub fn current_round_count(&self) -> u32 {
        self.current_round
    }

    /// Get round count for a specific state
    pub fn state_round_count(&self, state_name: &str) -> Option<u32> {
        self.state_round_counts.get(state_name).copied()
    }

    /// Get global round count
    pub fn global_round_count(&self) -> u32 {
        self.global_round_count
    }

    /// Get transition history
    pub fn transition_history(&self) -> &[StateTransition] {
        &self.transition_history
    }

    /// Get rounds since last visit to a specific state
    pub fn rounds_since_last_visit(&self, state_name: &str) -> Option<u32> {
        // Find the last transition to this state
        let mut rounds_since = 0;
        let mut found = false;

        for transition in self.transition_history.iter().rev() {
            if transition.to_state == state_name {
                found = true;
                break;
            }
            rounds_since += transition.round_count;
        }

        if found {
            Some(rounds_since)
        } else {
            // Never visited this state
            Some(self.global_round_count)
        }
    }
}