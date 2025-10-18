use std::fmt;
use super::state::{State, StateError, StateRoundTracker, StateTransition};

/// State machine for agent states and transitions
#[derive(Debug)]
pub struct StateMachine {
    states: Vec<State>,
    current_state_name: String,
    tracker: StateRoundTracker,
}

impl StateMachine {
    /// Create a new StateMachine with all states and initial state
    pub fn new(states: Vec<State>, init_state: &str) -> Result<Self, StateError> {
        // Verify that init_state exists in the provided states
        if !states.iter().any(|s| s.prompt_data.name == init_state) {
            return Err(StateError::StateNotFound(init_state.to_string()));
        }

        Ok(Self {
            states,
            tracker: StateRoundTracker::new(init_state.to_string()),
            current_state_name: init_state.to_string(),
        })
    }

    /// Register a new state
    pub fn register_state(&mut self, state: State) -> Result<(), StateError> {
        let name = state.prompt_data.name.clone();
        if self.find_state(&name).is_some() {
            return Err(StateError::ConfigurationError(
                format!("State '{}' already registered", name)
            ));
        }

        self.states.push(state);
        Ok(())
    }

    /// Find state by name
    fn find_state(&self, name: &str) -> Option<&State> {
        self.states.iter().find(|state| state.prompt_data.name == name)
    }

    /// Get current state
    pub fn current_state(&self) -> Result<&State, StateError> {
        self.find_state(&self.current_state_name)
            .ok_or_else(|| StateError::StateNotFound(self.current_state_name.clone()))
    }

    /// Get current state name
    pub fn current_state_name(&self) -> &str {
        &self.current_state_name
    }

    /// Get all registered states
    pub fn get_states(&self) -> &[State] {
        &self.states
    }

    /// Get all state names
    pub fn get_state_names(&self) -> Vec<String> {
        return self.get_states()
            .iter()
            .map(|state| state.prompt_data.name.clone())
            .collect()
    }

    /// Get state by name
    pub fn get_state(&self, name: &str) -> Option<&State> {
        self.find_state(name)
    }

    /// Get state by name (mutable)
    pub fn get_state_mut(&mut self, name: &str) -> Option<&mut State> {
        self.states.iter_mut().find(|state| state.prompt_data.name == name)
    }

    /// Increment round counter for current state
    pub fn increment_round(&mut self) {
        self.tracker.increment_round();
    }

    /// Get current round count
    pub fn current_round_count(&self) -> u32 {
        self.tracker.current_round_count()
    }

    /// Get global round count
    pub fn global_round_count(&self) -> u32 {
        self.tracker.global_round_count()
    }

    /// Get rounds since last visit to a specific state
    pub fn rounds_since_last_visit(&self, state_name: &str) -> Option<u32> {
        self.tracker.rounds_since_last_visit(state_name)
    }

    /// Transition to a new state (agent choice)
    pub fn transition_to(&mut self, state_name: String, reasoning: String) -> Result<u32, StateError> {
        if self.find_state(&state_name).is_none() {
            return Err(StateError::StateNotFound(state_name));
        }

        let previous_rounds = self.tracker.transition_to(
            state_name.clone(),
            reasoning
        );

        self.current_state_name = state_name;
        Ok(previous_rounds)
    }

    /// Get states that need forced transition
    pub fn get_forced_transitions(&self) -> Vec<(String, String)> {
        let mut forced_states = Vec::new();

        for state in &self.states {
            if let Some(max_interval) = state.max_round_interval {
                if let Some(rounds_since) = self.rounds_since_last_visit(&state.prompt_data.name) {
                    if rounds_since >= max_interval {
                        let reason = format!(
                            "Maximum round interval ({} rounds) reached for state '{}'. Last visited {} rounds ago.",
                            max_interval, state.prompt_data.name, rounds_since
                        );
                        forced_states.push((state.prompt_data.name.clone(), reason));
                    }
                }
            }
        }

        forced_states
    }

    /// Get states that should be recommended for transition
    pub fn get_recommended_transitions(&self) -> Vec<(String, String)> {
        let mut recommended_states = Vec::new();

        for state in &self.states {
            if let Some(max_interval) = state.max_round_interval {
                if let Some(rounds_since) = self.rounds_since_last_visit(&state.prompt_data.name) {
                    // Recommend when we're at 75% of max interval
                    let threshold = (max_interval as f64 * 0.75) as u32;
                    if rounds_since >= threshold && rounds_since < max_interval {
                        let reason = format!(
                            "Recommend transitioning to state '{}'. Current rounds since last visit: {} (max interval: {}).",
                            state.prompt_data.name, rounds_since, max_interval
                        );
                        recommended_states.push((state.prompt_data.name.clone(), reason));
                    }
                }
            }
        }

        recommended_states
    }

    /// Force transition to a state (system forced)
    pub fn force_transition_to(&mut self, state_name: String, reason: String) -> Result<u32, StateError> {
        if self.find_state(&state_name).is_none() {
            return Err(StateError::StateNotFound(state_name.clone()));
        }

        let previous_rounds = self.tracker.transition_to(
            state_name.clone(),
            reason
        );

        self.current_state_name = state_name;
        Ok(previous_rounds)
    }

    /// Get transition history
    pub fn transition_history(&self) -> &[StateTransition] {
        self.tracker.transition_history()
    }

    /// Execute current state with given context
    pub async fn execute_current_state(&self, context: &str) -> anyhow::Result<String> {
        let current_state = self.current_state()?;
        current_state.execute(context).await
    }

    /// Get state information for LLM tools
    pub fn get_state_info(&self) -> Vec<StateInfo> {
        self.states.iter().map(|state| {
            StateInfo {
                name: state.prompt_data.name.clone(),
                description: state.prompt_data.description.clone(),
                current_rounds: self.tracker.state_round_count(&state.prompt_data.name).unwrap_or(0),
                rounds_since_last_visit: self.rounds_since_last_visit(&state.prompt_data.name),
                max_interval: state.max_round_interval,
            }
        }).collect()
    }
}

/// State information for LLM consumption
#[derive(Debug, Clone)]
pub struct StateInfo {
    pub name: String,
    pub description: String,
    pub current_rounds: u32,
    pub rounds_since_last_visit: Option<u32>,
    pub max_interval: Option<u32>,
}

impl fmt::Display for StateInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "State '{}': {} | Current rounds: {} | Rounds since last visit: {:?} | Max interval: {:?}",
               self.name, self.description, self.current_rounds,
               self.rounds_since_last_visit, self.max_interval)
    }
}