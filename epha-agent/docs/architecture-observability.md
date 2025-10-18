# EPHA-Agent Observability & Control API

## Overview

Comprehensive monitoring and dynamic control system for autonomous agents, providing real-time insights and configuration capabilities.

## Metrics Collection System

### Core Metrics Categories

#### 1. Loop Performance Metrics
```rust
#[derive(Debug, Serialize)]
pub struct LoopMetrics {
    pub loop_id: LoopId,
    pub timestamp: Instant,
    pub total_duration: Duration,
    pub phase_timings: PhaseTimingMetrics,
    pub llm_requests: LLMRequestMetrics,
    pub resource_usage: ResourceMetrics,
}
```

#### 2. LLM Request Tracking
```rust
#[derive(Debug, Serialize)]
pub struct LLMRequestMetrics {
    pub total_requests: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub total_tokens_used: u64,
    pub average_response_time: Duration,
    pub requests_by_phase: HashMap<LoopPhase, usize>,
    pub tokens_by_phase: HashMap<LoopPhase, u64>,
}
```

#### 3. Dynamic Phase Performance Analysis
```rust
#[derive(Debug, Serialize)]
pub struct PhaseTimingMetrics {
    pub phases: HashMap<String, PhaseStats>,
    pub execution_order: Vec<String>,
    pub total_execution_time: Duration,
    pub dependency_resolution_time: Duration,
    pub custom_phases: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PhaseStats {
    pub phase_name: String,
    pub phase_type: String,
    pub duration: Duration,
    pub request_count: usize,
    pub success_rate: f64,
    pub tokens_consumed: u64,
    pub outputs_produced: Vec<String>,
    pub dependencies_satisfied: Vec<String>,
    pub bottlenecks: Vec<String>,
    pub retry_count: usize,
    pub resource_usage: ResourceUsage,
}

#[derive(Debug, Serialize)]
pub struct ResourceUsage {
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub network_io_mb: f64,
    pub disk_io_mb: f64,
}
```

### Metrics Collection Points

#### Dynamic Per-Phase Instrumentation
```rust
// Automatic instrumentation for any phase
#[instrumentation]
async fn execute_phase_with_metrics<P: Phase>(
    phase: &mut P,
    context: &mut PhaseContext
) -> PhaseResult {
    let phase_start = Instant::now();
    let phase_name = phase.name();

    // Record phase start
    context.metrics.start_phase(&phase_name).await;

    // Execute phase with error handling
    let result = phase.execute(context).await;

    // Record phase completion
    let duration = phase_start.elapsed();
    let resource_usage = context.get_resource_usage().await;

    context.metrics.record_phase_completion(
        &phase_name,
        duration,
        &result,
        resource_usage
    ).await;

    result
}
```

#### Request-Level Tracking
```rust
struct LLMRequestTracker {
    phase: LoopPhase,
    request_type: RequestType,
    start_time: Instant,
}

impl LLMRequestTracker {
    async fn track_request<F, R>(&self, operation: F) -> Result<R, LLMError>
    where
        F: Future<Output = Result<R, LLMError>>,
    {
        let result = operation.await;
        let response_time = self.start_time.elapsed();

        // Record request metrics
        match &result {
            Ok(response) => {
                self.metrics.record_successful_request(
                    self.phase,
                    self.request_type,
                    response.tokens_used(),
                    response_time
                ).await;
            }
            Err(error) => {
                self.metrics.record_failed_request(
                    self.phase,
                    self.request_type,
                    error.to_string(),
                    response_time
                ).await;
            }
        }

        result
    }
}
```

## Dynamic Control System

### Rate Limiting Control

#### Window-Based Rate Limiter
```rust
pub struct RateController {
    window_duration: Duration,
    max_loops_per_window: usize,
    loop_history: VecDeque<Instant>,
    current_state: RateState,
}

#[derive(Debug)]
pub enum RateState {
    Normal,
    Throttled { wait_time: Duration },
    Paused,
    Stopped,
}

impl RateController {
    pub async fn wait_if_needed(&mut self) -> Result<(), RateLimitError> {
        match self.current_state {
            RateState::Paused => {
                // Wait for resume signal
                self.wait_for_resume().await
            }
            RateState::Throttled { wait_time } => {
                tokio::time::sleep(wait_time).await;
                Ok(())
            }
            RateState::Normal => {
                self.check_window_limit().await
            }
            RateState::Stopped => {
                Err(RateLimitError::Stopped)
            }
        }
    }

    pub async fn update_rate_limit(&mut self,
        window_secs: u64,
        max_loops: usize
    ) -> Result<(), ConfigError> {
        self.window_duration = Duration::from_secs(window_secs);
        self.max_loops_per_window = max_loops;
        self.cleanup_old_history().await;
        Ok(())
    }
}
```

### Configuration Management

#### Dynamic Configuration API
```rust
#[async_trait]
pub trait AgentController {
    // Rate control
    async fn set_rate_limit(&mut self, window_secs: u64, max_loops: usize) -> Result<(), ControllerError>;
    async fn get_current_rate(&self) -> RateConfig;
    async fn pause_agent(&mut self) -> Result<(), ControllerError>;
    async fn resume_agent(&mut self) -> Result<(), ControllerError>;

    // Behavioral configuration
    async fn update_behavior_config(&mut self, config: BehaviorConfig) -> Result<(), ControllerError>;
    async fn get_behavior_config(&self) -> BehaviorConfig;

    // Resource limits
    async fn set_resource_limits(&mut self, limits: ResourceLimits) -> Result<(), ControllerError>;
    async fn get_resource_usage(&self) -> ResourceUsage;

    // Metrics access
    async fn get_current_metrics(&self) -> LoopMetrics;
    async fn get_metrics_history(&self, duration: Duration) -> Vec<LoopMetrics>;
    async fn get_performance_summary(&self) -> PerformanceSummary;
}

#[derive(Debug, Clone, Serialize)]
pub struct BehaviorConfig {
    pub exploration_tendency: f32,    // 0.0 - 1.0
    pub risk_tolerance: f32,          // 0.0 - 1.0
    pub learning_rate: f32,           // 0.0 - 1.0
    pub social_engagement: f32,       // 0.0 - 1.0
    pub goal_persistence: f32,        // 0.0 - 1.0
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceLimits {
    pub max_memory_mb: usize,
    pub max_cpu_percent: f32,
    pub max_llm_requests_per_minute: usize,
    pub max_tokens_per_hour: u64,
}
```

## REST API Interface

### Control Endpoints

#### Rate Management
```http
# Get current rate configuration
GET /api/agents/{agent_id}/rate

Response:
{
  "window_seconds": 10,
  "max_loops_per_window": 50,
  "current_state": "normal",
  "current_loops_in_window": 23
}

# Update rate configuration
PUT /api/agents/{agent_id}/rate
Content-Type: application/json

{
  "window_seconds": 15,
  "max_loops_per_window": 30
}

# Pause agent execution
POST /api/agents/{agent_id}/pause

# Resume agent execution
POST /api/agents/{agent_id}/resume
```

#### Configuration Management
```http
# Update behavior configuration
PUT /api/agents/{agent_id}/config/behavior
Content-Type: application/json

{
  "exploration_tendency": 0.7,
  "risk_tolerance": 0.3,
  "learning_rate": 0.8,
  "social_engagement": 0.6,
  "goal_persistence": 0.9
}

# Update resource limits
PUT /api/agents/{agent_id}/config/resources
Content-Type: application/json

{
  "max_memory_mb": 512,
  "max_cpu_percent": 80.0,
  "max_llm_requests_per_minute": 100,
  "max_tokens_per_hour": 10000
}
```

### Monitoring Endpoints

#### Metrics Access
```http
# Get current loop metrics
GET /api/agents/{agent_id}/metrics/current

Response:
{
  "loop_id": "loop_123",
  "timestamp": "2025-10-08T10:30:00Z",
  "total_duration_ms": 1250,
  "phase_timings": {
    "perception": { "duration_ms": 200, "requests": 3 },
    "cognition": { "duration_ms": 600, "requests": 8 },
    "action": { "duration_ms": 300, "requests": 2 },
    "reflection": { "duration_ms": 150, "requests": 1 }
  },
  "llm_requests": {
    "total": 14,
    "successful": 14,
    "total_tokens": 2847,
    "average_response_time_ms": 180
  }
}

# Get metrics history
GET /api/agents/{agent_id}/metrics/history?duration=1h&granularity=1m

# Get performance summary
GET /api/agents/{agent_id}/metrics/summary

Response:
{
  "loops_per_minute": 4.2,
  "average_loop_time_ms": 1100,
  "requests_per_loop": 12.5,
  "tokens_per_loop": 2450,
  "success_rate": 0.98,
  "phase_breakdown": {
    "perception_percentage": 18,
    "cognition_percentage": 55,
    "action_percentage": 27
  }
}
```

#### Real-time Monitoring
```http
# WebSocket for real-time metrics
WS /api/agents/{agent_id}/metrics/stream

# Real-time events
WS /api/agents/{agent_id}/events/stream
```

## Integration Examples

### Agent Implementation with Control
```rust
pub struct ControlledAgent {
    id: AgentId,
    state: AgentState,
    controller: Box<dyn AgentController>,
    metrics: MetricsCollector,
    rate_limiter: RateController,
}

impl ControlledAgent {
    pub async fn run(&mut self) -> Result<(), AgentError> {
        loop {
            // Check rate limits
            self.rate_limiter.wait_if_needed().await?;

            // Start metrics collection
            self.metrics.start_loop();

            // Execute agent phases with instrumentation
            self.execute_perception_phase().await?;
            self.execute_cognition_phase().await?;
            self.execute_action_phase().await?;
            self.execute_reflection_phase().await?;

            // Complete metrics collection
            self.metrics.complete_loop().await;
        }
    }

    pub async fn handle_control_command(&mut self, command: ControlCommand) -> Result<(), ControllerError> {
        match command {
            ControlCommand::UpdateRate { window, max_loops } => {
                self.controller.set_rate_limit(window, max_loops).await?;
                self.rate_limiter.update_rate_limit(window, max_loops).await?;
            }
            ControlCommand::Pause => {
                self.controller.pause_agent().await?;
            }
            ControlCommand::Resume => {
                self.controller.resume_agent().await?;
            }
            ControlCommand::UpdateConfig(config) => {
                self.controller.update_behavior_config(config).await?;
                self.update_behavior(config).await?;
            }
        }
        Ok(())
    }
}
```

## Performance Considerations

### Metrics Overhead
- **Low impact design**: Metrics collection adds <1% overhead
- **Async recording**: Non-blocking metrics storage
- **Batch processing**: Periodic batch writes to storage

### Control Latency
- **Immediate effect**: Configuration changes apply within one loop cycle
- **Graceful degradation**: Control interface remains responsive during high load
- **Fallback mechanisms**: Automatic recovery from control failures

This observability and control system provides comprehensive insight into agent behavior while maintaining minimal performance overhead and enabling real-time dynamic configuration.