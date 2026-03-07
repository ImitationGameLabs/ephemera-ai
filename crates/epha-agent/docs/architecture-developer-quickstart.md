# EPHA-Agent Developer Quickstart

## Core Concepts

### Agent Loop Structure
```rust
async fn agent_loop(&mut self) {
    loop {
        // 1. Perception - Check for events
        self.perceive_environment().await;

        // 2. Cognition - Make decisions
        self.process_thoughts().await;

        // 3. Action - Execute decisions
        self.take_actions().await;

        // 4. Reflection - Learn and adjust
        self.reflect().await;

        // 5. Maintenance - Resource management
        self.maintenance().await;
    }
}
```

### Event Integration
```rust
// Events are processed non-blocking
async fn perceive_environment(&mut self) {
    let critical_events = self.critical_rx.try_recv_all()?;
    let priority_events = self.priority_rx.try_recv_all()?;

    // Process without blocking the loop
    for event in critical_events {
        self.handle_immediately(event).await;
    }

    // Queue others for processing
    self.queue_events(priority_events).await;
}
```

## Implementation Steps

### 1. Create Dynamic Agent with Phases

```rust
use epha_agent::*;

struct MyAgent {
    id: AgentId,
    phase_executor: DynamicPhaseExecutor,
    controller: AgentController,
    metrics: MetricsCollector,
    rate_limiter: RateController,
}

impl MyAgent {
    async fn new() -> Self {
        let mut phase_executor = DynamicPhaseExecutor::new();

        // Configure phases dynamically
        phase_executor.add_phase(Box::new(PerceptionPhase::new())).unwrap();
        phase_executor.add_phase(Box::new(CognitionPhase::new())).unwrap();
        phase_executor.add_phase(Box::new(ActionPhase::new())).unwrap();
        phase_executor.add_phase(Box::new(ReflectionPhase::new())).unwrap();

        Self {
            id: agent_id,
            phase_executor,
            controller: AgentController::new(),
            metrics: MetricsCollector::new(agent_id),
            rate_limiter: RateController::new(10, 50),
        }
    }

    async fn run(&mut self) -> Result<(), AgentError> {
        loop {
            // Rate limiting
            self.rate_limiter.wait_if_needed().await?;

            // Create phase context
            let mut context = self.create_phase_context().await;

            // Execute all phases dynamically
            self.phase_executor.execute_all_phases(&mut context).await?;

            // Complete metrics collection
            self.metrics.complete_loop().await;
        }
    }

    async fn create_phase_context(&self) -> PhaseContext {
        PhaseContext {
            agent_id: self.id,
            agent_state: self.get_agent_state().await,
            loop_id: self.generate_loop_id(),
            // ... other context fields
        }
    }
}
```

### 2. Create Custom Phase

```rust
// Example: Custom Analysis Phase
pub struct AnalysisPhase {
    analyzer: Box<dyn Analyzer>,
    config: AnalysisConfig,
}

#[async_trait]
impl Phase for AnalysisPhase {
    fn name(&self) -> &str { "analysis" }

    fn dependencies(&self) -> Vec<String> {
        vec!["perception".to_string()]
    }

    async fn execute(&mut self, context: &mut PhaseContext) -> PhaseResult {
        // Get data from perception phase
        let perception_data = context.get_shared_data("perception_results").await?;

        // Perform custom analysis
        let analysis_results = self.analyzer.analyze(&perception_data).await?;

        // Store results for other phases
        context.set_shared_data("analysis_results", analysis_results).await;

        PhaseResult::Success {
            outputs: vec!["analysis_results".to_string()],
            metadata: HashMap::from([
                ("analysis_time_ms".to_string(), "150".to_string()),
                ("confidence_score".to_string(), "0.92".to_string()),
            ]),
        }
    }

    async fn on_error(&mut self, error: &PhaseError) -> PhaseErrorAction {
        match error {
            PhaseError::AnalysisFailed(_) => PhaseErrorAction::Retry,
            PhaseError::ResourceExhausted => PhaseErrorAction::Skip,
            _ => PhaseErrorAction::Abort,
        }
    }
}

// Add custom phase to agent
impl MyAgent {
    async fn add_custom_analysis_phase(&mut self) -> Result<(), AgentError> {
        let analysis_phase = AnalysisPhase::new(AnalysisConfig::default());
        self.phase_executor.add_phase(Box::new(analysis_phase))?;
        Ok(())
    }
}
```

### 3. Configure Phases from YAML

```rust
// Load phase configuration from file
impl MyAgent {
    async fn from_config_file(config_path: &str) -> Result<Self, AgentError> {
        let config_file = PhaseConfigFile::load_from_file(config_path)?;
        let mut phase_executor = DynamicPhaseExecutor::from_config(config_file)?;

        Ok(Self {
            id: config_file.agent_id,
            phase_executor,
            controller: AgentController::new(),
            metrics: MetricsCollector::new(config_file.agent_id),
            rate_limiter: RateController::new(10, 50),
        })
    }
}

// Example YAML configuration
/*
phases:
  perception:
    type: "PerceptionPhase"
    enabled: true
    config:
      event_sources: ["sensor", "user", "system"]

  custom_analysis:
    type: "AnalysisPhase"
    enabled: true
    dependencies: ["perception"]
    config:
      analyzer_type: "sentiment"
      confidence_threshold: 0.8

  cognition:
    type: "CognitionPhase"
    enabled: true
    dependencies: ["perception", "custom_analysis"]
*/

### 2. Define Agent State
```rust
struct AgentState {
    // Persistent identity
    identity: IdentityState,

    // Current cognitive state
    goals: Vec<Goal>,
    working_memory: WorkingMemory,

    // Emotional/motivational state
    motivation: f32,
    satisfaction: f32,

    // Resource state
    energy: f32,
    cognitive_load: f32,
}
```

### 3. Implement Decision Making
```rust
impl MyAgent {
    async fn process_thoughts(&mut self) {
        // Evaluate current goals
        let prioritized_goals = self.prioritize_goals().await;

        // Select next action
        let action = self.select_action(prioritized_goals).await;

        // Update state
        self.planned_action = Some(action);
    }

    async fn select_action(&self, goals: Vec<Goal>) -> Action {
        // Consider motivation, energy, context
        // Return best action for current state
    }
}
```

### 4. Handle Events
```rust
impl MyAgent {
    async fn handle_immediately(&mut self, event: Event) {
        match event {
            Event::UserMessage { content } => {
                self.process_user_input(content).await;
            }
            Event::AgentMessage { from, content } => {
                self.process_agent_message(from, content).await;
            }
            Event::SystemAlert { alert_type } => {
                self.handle_system_alert(alert_type).await;
            }
        }
    }
}
```

## Key Patterns

### Non-blocking I/O
```rust
// Always use non-blocking operations
let events = self.receiver.try_recv_all()?;  // Don't block
if let Some(data) = self.cache.get(key) {    // Try-get first
    return data;
}
let data = self.load_from_storage(key).await; // Then async load
```

### State Updates
```rust
// Update state atomically
async fn update_motivation(&mut self, delta: f32) {
    self.state.motivation = (self.state.motivation + delta).clamp(0.0, 1.0);
    self.state.last_update = Utc::now();
}
```

### Goal Management
```rust
async fn prioritize_goals(&self) -> Vec<Goal> {
    let mut goals = self.state.goals.clone();

    // Sort by urgency, importance, feasibility
    goals.sort_by(|a, b| {
        b.urgency.partial_cmp(&a.urgency).unwrap()
        .then(b.importance.partial_cmp(&a.importance).unwrap())
    });

    // Return top N goals based on cognitive capacity
    let capacity = self.calculate_cognitive_capacity();
    goals.into_iter().take(capacity).collect()
}
```

## Testing Your Agent

### Unit Testing
```rust
#[tokio::test]
async fn test_agent_decision() {
    let mut agent = MyAgent::new().await;

    // Set up state
    agent.state.motivation = 0.8;
    agent.state.energy = 0.9;

    // Test decision making
    let action = agent.select_action(vec![goal1, goal2]).await;
    assert!(matches!(action, Action::Explore));
}
```

### Integration Testing
```rust
#[tokio::test]
async fn test_event_processing() {
    let (tx, rx) = create_event_channel();
    let mut agent = MyAgent::with_receiver(rx).await;

    // Send test event
    let event = Event::UserMessage { content: "test".to_string() };
    tx.send(event).unwrap();

    // Process event
    agent.perceive_environment().await;

    // Verify response
    assert!(agent.has_processed_user_input());
}
```

## Rate Limiting Implementation

### Window-Based Rate Control
```rust
impl MyAgent {
    async fn configure_rate_limiting(&mut self, window_secs: u64, max_loops: usize) {
        self.rate_limiter.update_rate_limit(window_secs, max_loops).await;
        self.controller.set_rate_limit(window_secs, max_loops).await.unwrap();
    }

    async fn pause_agent(&mut self) -> Result<(), ControllerError> {
        self.controller.pause_agent().await
    }

    async fn resume_agent(&mut self) -> Result<(), ControllerError> {
        self.controller.resume_agent().await
    }
}
```

### Dynamic Configuration
```rust
impl MyAgent {
    async fn update_behavior(&mut self, config: BehaviorConfig) -> Result<(), ControllerError> {
        // Update internal behavior parameters
        self.state.exploration_tendency = config.exploration_tendency;
        self.state.risk_tolerance = config.risk_tolerance;
        self.state.learning_rate = config.learning_rate;

        // Persist configuration
        self.controller.update_behavior_config(config).await
    }

    async fn set_resource_limits(&mut self, limits: ResourceLimits) -> Result<(), ControllerError> {
        self.controller.set_resource_limits(limits).await
    }
}
```

## Metrics Collection Implementation

### Phase Instrumentation
```rust
impl MyAgent {
    async fn execute_perception_phase(&mut self) -> Result<(), AgentError> {
        let phase_start = Instant::now();
        self.metrics.start_phase(LoopPhase::Perception).await;

        // Execute perception logic
        let events_processed = self.process_environmental_events().await;
        let llm_requests = self.analyze_events(&events_processed).await?;

        // Record phase metrics
        self.metrics.record_phase_completion(
            LoopPhase::Perception,
            phase_start.elapsed(),
            llm_requests.len(),
            llm_requests.iter().map(|r| r.tokens_used).sum()
        ).await;

        Ok(())
    }

    async fn execute_cognition_phase(&mut self) -> Result<(), AgentError> {
        let phase_start = Instant::now();
        self.metrics.start_phase(LoopPhase::Cognition).await;

        // Execute cognition logic with LLM tracking
        let decision_requests = self.evaluate_goals().await?;
        let planning_requests = self.plan_actions().await?;

        // Record cognitive metrics
        let total_requests = decision_requests.len() + planning_requests.len();
        let total_tokens = decision_requests.iter()
            .chain(planning_requests.iter())
            .map(|r| r.tokens_used)
            .sum();

        self.metrics.record_phase_completion(
            LoopPhase::Cognition,
            phase_start.elapsed(),
            total_requests,
            total_tokens
        ).await;

        Ok(())
    }
}
```

### LLM Request Tracking
```rust
impl MyAgent {
    async fn make_llm_request_with_tracking(&mut self,
        phase: LoopPhase,
        request_type: RequestType,
        prompt: &str
    ) -> Result<LLMResponse, LLMError> {
        let request_start = Instant::now();

        let result = self.llm_client.generate(prompt).await;
        let response_time = request_start.elapsed();

        match &result {
            Ok(response) => {
                self.metrics.record_successful_request(
                    phase,
                    request_type,
                    response.tokens_used,
                    response_time
                ).await;
            }
            Err(error) => {
                self.metrics.record_failed_request(
                    phase,
                    request_type,
                    error.to_string(),
                    response_time
                ).await;
            }
        }

        result
    }
}
```

## Control API Usage

### REST API Integration
```rust
use reqwest;

#[async_trait]
impl RemoteAgentController {
    async fn configure_agent_rate(&self, agent_id: &str, window_secs: u64, max_loops: usize) -> Result<(), ConfigError> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/agents/{}/rate", self.base_url, agent_id);

        let response = client
            .put(&url)
            .json(&json!({
                "window_seconds": window_secs,
                "max_loops_per_window": max_loops
            }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ConfigError::ApiError(response.status()))
        }
    }

    async fn get_agent_metrics(&self, agent_id: &str) -> Result<LoopMetrics, MetricsError> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/agents/{}/metrics/current", self.base_url, agent_id);

        let response = client.get(&url).send().await?;
        let metrics: LoopMetrics = response.json().await?;

        Ok(metrics)
    }
}
```

### Real-time Monitoring
```rust
use tokio_tungstenite::{connect_async, tungstenite::Message};

impl MyAgent {
    async fn start_monitoring_stream(&self) -> Result<(), MonitoringError> {
        let url = format!("{}/api/agents/{}/metrics/stream", self.base_url, self.id);
        let (ws_stream, _) = connect_async(&url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Send subscription request
        write.send(Message::Text(json!({
            "type": "subscribe",
            "metrics": ["current", "summary", "errors"]
        }).to_string())).await?;

        // Process incoming metrics
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let metrics: LoopMetrics = serde_json::from_str(&text)?;
                    self.handle_realtime_metrics(metrics).await?;
                }
                Ok(Message::Close(_)) => break,
                Err(e) => return Err(MonitoringError::WebSocket(e)),
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_realtime_metrics(&self, metrics: LoopMetrics) -> Result<(), MonitoringError> {
        // Check for performance issues
        if metrics.total_duration > Duration::from_secs(5) {
            warn!("Slow loop detected: {:?}", metrics.total_duration);
            self.trigger_performance_alert(metrics).await?;
        }

        // Check LLM request rates
        if metrics.llm_requests.total_requests > 20 {
            warn!("High LLM request rate: {}", metrics.llm_requests.total_requests);
            self.suggest_rate_adjustment().await?;
        }

        Ok(())
    }
}
```

## Performance Tips

1. **Minimize Blocking Operations**: Use async/await throughout
2. **Batch Process Events**: Handle multiple events together
3. **Cache Frequently Used Data**: Reduce redundant computations
4. **Adaptive Rate Limiting**: Adjust based on system load and complexity
5. **Efficient Metrics**: Use circular buffers and batch writes
6. **Use Efficient Data Structures**: DashMap, ArrayDeque for concurrent access

## Control Interface Examples

### Command Line Interface
```bash
# Configure rate limiting
curl -X PUT http://localhost:8080/api/agents/agent-001/rate \
  -H "Content-Type: application/json" \
  -d '{"window_seconds": 15, "max_loops_per_window": 30}'

# Pause agent
curl -X POST http://localhost:8080/api/agents/agent-001/pause

# Update behavior
curl -X PUT http://localhost:8080/api/agents/agent-001/config/behavior \
  -H "Content-Type: application/json" \
  -d '{
    "exploration_tendency": 0.8,
    "risk_tolerance": 0.4,
    "learning_rate": 0.9
  }'

# Get current metrics
curl http://localhost:8080/api/agents/agent-001/metrics/current
```

### Programmatic Control
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let controller = RemoteAgentController::new("http://localhost:8080");

    // Configure multiple agents
    for agent_id in ["agent-001", "agent-002", "agent-003"] {
        controller.configure_agent_rate(agent_id, 10, 50).await?;
        controller.update_behavior_config(agent_id, BehaviorConfig {
            exploration_tendency: 0.7,
            risk_tolerance: 0.3,
            learning_rate: 0.8,
            social_engagement: 0.6,
            goal_persistence: 0.9,
        }).await?;
    }

    // Monitor performance
    let metrics = controller.get_agent_metrics("agent-001").await?;
    println!("Current performance: {:?}", metrics);

    Ok(())
}
```

## Common Pitfalls

- ❌ Blocking the agent loop with synchronous operations
- ❌ Ignoring event priorities
- ❌ Not managing cognitive load properly
- ❌ State inconsistencies in concurrent operations
- ❌ Infinite loops without back-pressure control

## Debugging

### Logging State Changes
```rust
debug!("Agent {} motivation: {:.2}", self.id, self.state.motivation);
info!("Selected action: {:?}", action);
warn!("High cognitive load: {:.2}", self.state.cognitive_load);
```

### Performance Monitoring
```rust
let start = Instant::now();
self.process_thoughts().await;
let duration = start.elapsed();
trace!("Thought processing took: {:?}", duration);
```

---

*For complete API documentation, see the developer guide. For architectural decisions, see the core architecture document.*