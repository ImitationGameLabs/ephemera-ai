# EPHA-Agent Phase Architecture

## Overview

The EPHA-Agent phase system provides a flexible, pluggable architecture for agent cognitive cycles, moving beyond hardcoded loops to a dynamic, configurable approach.

## Phase Trait Definition

### Core Interface

```rust
#[async_trait]
pub trait Phase: Send + Sync + 'static {
    // Basic Information
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn phase_type(&self) -> PhaseType;

    // Dependencies
    fn dependencies(&self) -> Vec<String>;
    fn provides(&self) -> Vec<String>;

    // Execution Interface
    async fn execute(&mut self, context: &mut PhaseContext) -> PhaseResult;
    async fn pre_execute(&mut self, context: &PhaseContext) -> PhaseResult;
    async fn post_execute(&mut self, context: &PhaseContext) -> PhaseResult;

    // Error Handling
    async fn on_error(&mut self, error: &PhaseError) -> PhaseErrorAction;
    async fn can_retry(&self, error: &PhaseError) -> bool;
    async fn get_retry_delay(&self, attempt: usize) -> Duration;

    // Resource Management
    async fn get_resource_requirements(&self) -> ResourceRequirements;
    async fn cleanup(&mut self) -> Result<(), PhaseError>;

    // Lifecycle
    async fn initialize(&mut self, config: &PhaseConfig) -> Result<(), PhaseError>;
    async fn shutdown(&mut self) -> Result<(), PhaseError>;
}
```

### Phase Types

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum PhaseType {
    Perception,    // Environmental monitoring
    Cognition,     // Decision making
    Action,        // Tool execution
    Reflection,    // Learning and adjustment
    Maintenance,   // Health and resources
    Custom(String), // User-defined phase types
}
```

### Phase Context

```rust
pub struct PhaseContext {
    // Agent State
    pub agent_id: AgentId,
    pub agent_state: Arc<RwLock<AgentState>>,
    pub loop_id: LoopId,

    // Data Exchange
    pub shared_data: Arc<RwLock<HashMap<String, SharedData>>>,
    pub event_queue: Arc<Mutex<VecDeque<Event>>>,

    // External Services
    pub llm_client: Arc<dyn LLMClient>,
    pub tool_registry: Arc<ToolRegistry>,
    pub memory_store: Arc<dyn MemoryStore>,

    // Monitoring
    pub metrics_collector: Arc<Mutex<MetricsCollector>>,
    pub logger: Arc<dyn Logger>,

    // Configuration
    pub phase_config: HashMap<String, PhaseConfig>,
    pub execution_strategy: ExecutionStrategy,
}
```

## Core Phase Implementations

### 1. Perception Phase

**Purpose**: Environmental monitoring, event processing, context awareness

```rust
pub struct PerceptionPhase {
    config: PerceptionConfig,
    event_filters: Vec<Box<dyn EventFilter>>,
    attention_manager: AttentionManager,
}

impl Phase for PerceptionPhase {
    fn name(&self) -> &str { "perception" }

    async fn execute(&mut self, context: &mut PhaseContext) -> PhaseResult {
        // 1. Gather environmental events
        let events = self.collect_events(context).await?;

        // 2. Filter relevant events
        let filtered_events = self.filter_events(&events, context).await?;

        // 3. Update working memory
        self.update_working_memory(filtered_events, context).await?;

        // 4. Identify priority stimuli
        let priority_stimuli = self.identify_priorities(context).await?;

        PhaseResult::Success {
            outputs: vec!["priority_stimuli".to_string()],
            metadata: HashMap::from([
                ("events_processed".to_string(), events.len().to_string()),
                ("priority_count".to_string(), priority_stimuli.len().to_string()),
            ]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerceptionConfig {
    pub event_sources: Vec<String>,
    pub filter_policies: Vec<FilterPolicy>,
    pub attention_capacity: usize,
    pub refresh_rate_ms: u64,
}
```

### 2. Cognition Phase

**Purpose**: Goal evaluation, decision making, planning, reasoning

```rust
pub struct CognitionPhase {
    config: CognitionConfig,
    goal_manager: Arc<Mutex<GoalManager>>,
    decision_engine: Box<dyn DecisionEngine>,
    planning_system: PlanningSystem,
}

impl Phase for CognitionPhase {
    fn name(&self) -> &str { "cognition" }

    fn dependencies(&self) -> Vec<String> {
        vec!["perception".to_string()]
    }

    async fn execute(&mut self, context: &mut PhaseContext) -> PhaseResult {
        // 1. Retrieve priority stimuli from perception
        let stimuli = self.get_priority_stimuli(context).await?;

        // 2. Evaluate current goals against stimuli
        let goal_evaluations = self.evaluate_goals(&stimuli, context).await?;

        // 3. Make decisions
        let decisions = self.make_decisions(&goal_evaluations, context).await?;

        // 4. Create action plans
        let action_plans = self.create_plans(&decisions, context).await?;

        PhaseResult::Success {
            outputs: vec!["action_plans".to_string()],
            metadata: HashMap::from([
                ("decisions_made".to_string(), decisions.len().to_string()),
                ("plans_created".to_string(), action_plans.len().to_string()),
            ]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CognitionConfig {
    pub max_concurrent_goals: usize,
    pub decision_model: String,
    pub planning_horizon: Duration,
    pub risk_tolerance: f32,
}
```

### 3. Action Phase

**Purpose**: Tool execution, communication, environmental interaction

```rust
pub struct ActionPhase {
    config: ActionConfig,
    tool_executor: Arc<ToolExecutor>,
    action_scheduler: ActionScheduler,
    safety_checker: SafetyChecker,
}

impl Phase for ActionPhase {
    fn name(&self) -> &str { "action" }

    fn dependencies(&self) -> Vec<String> {
        vec!["cognition".to_string()]
    }

    async fn execute(&mut self, context: &mut PhaseContext) -> PhaseResult {
        // 1. Retrieve action plans from cognition
        let plans = self.get_action_plans(context).await?;

        // 2. Safety check all actions
        let safe_plans = self.safety_check(&plans, context).await?;

        // 3. Schedule and execute actions
        let execution_results = self.execute_actions(&safe_plans, context).await?;

        // 4. Record action outcomes
        self.record_outcomes(&execution_results, context).await?;

        PhaseResult::Success {
            outputs: vec!["action_results".to_string()],
            metadata: HashMap::from([
                ("actions_executed".to_string(), execution_results.len().to_string()),
                ("success_rate".to_string(),
                 format!("{:.2}", self.calculate_success_rate(&execution_results))),
            ]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActionConfig {
    pub parallel_execution: bool,
    pub max_concurrent_actions: usize,
    pub timeout_ms: u64,
    pub retry_policy: RetryPolicy,
}
```

### 4. Reflection Phase

**Purpose**: Learning, strategy adjustment, performance analysis

```rust
pub struct ReflectionPhase {
    config: ReflectionConfig,
    learning_system: Box<dyn LearningSystem>,
    performance_analyzer: PerformanceAnalyzer,
    strategy_adjuster: StrategyAdjuster,
}

impl Phase for ReflectionPhase {
    fn name(&self) -> &str { "reflection" }

    fn dependencies(&self) -> Vec<String> {
        vec!["action".to_string()]
    }

    async fn execute(&mut self, context: &mut PhaseContext) -> PhaseResult {
        // 1. Retrieve action outcomes
        let outcomes = self.get_action_outcomes(context).await?;

        // 2. Analyze performance
        let performance_metrics = self.analyze_performance(&outcomes, context).await?;

        // 3. Extract learning insights
        let insights = self.extract_insights(&outcomes, &performance_metrics, context).await?;

        // 4. Update strategies and models
        self.update_strategies(&insights, context).await?;

        // 5. Consolidate learning
        self.consolidate_learning(context).await?;

        PhaseResult::Success {
            outputs: vec!["learning_updates".to_string()],
            metadata: HashMap::from([
                ("insights_extracted".to_string(), insights.len().to_string()),
                ("performance_score".to_string(),
                 format!("{:.3}", performance_metrics.overall_score)),
            ]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReflectionConfig {
    pub learning_rate: f32,
    pub memory_consolidation_interval: Duration,
    pub strategy_update_threshold: f32,
    pub performance_tracking_window: Duration,
}
```

### 5. Maintenance Phase

**Purpose**: Health monitoring, resource management, system optimization

```rust
pub struct MaintenancePhase {
    config: MaintenanceConfig,
    health_checker: HealthChecker,
    resource_manager: ResourceManager,
    memory_optimizer: MemoryOptimizer,
}

impl Phase for MaintenancePhase {
    fn name(&self) -> &str { "maintenance" }

    async fn execute(&mut self, context: &mut PhaseContext) -> PhaseResult {
        // 1. System health check
        let health_status = self.health_checker.check_system(context).await?;

        // 2. Resource usage analysis
        let resource_usage = self.resource_manager.analyze_usage(context).await?;

        // 3. Memory optimization
        let optimization_results = self.memory_optimizer.optimize(context).await?;

        // 4. Performance tuning
        self.tune_performance(&resource_usage, context).await?;

        PhaseResult::Success {
            outputs: vec!["maintenance_report".to_string()],
            metadata: HashMap::from([
                ("health_score".to_string(), health_status.score.to_string()),
                ("memory_optimized_mb".to_string(),
                 optimization_results.memory_freed_mb.to_string()),
            ]),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MaintenanceConfig {
    pub health_check_interval: Duration,
    pub memory_cleanup_threshold: f32,
    pub performance_tuning_enabled: bool,
    pub auto_recovery_enabled: bool,
}
```

## Dynamic Phase Execution

### Phase Executor

```rust
pub struct DynamicPhaseExecutor {
    phases: HashMap<String, Box<dyn Phase>>,
    execution_graph: ExecutionGraph,
    execution_strategy: ExecutionStrategy,
    error_handler: Box<dyn ErrorHandler>,
}

impl DynamicPhaseExecutor {
    // Phase Management
    pub fn add_phase(&mut self, phase: Box<dyn Phase>) -> Result<(), PhaseError>;
    pub fn remove_phase(&mut self, phase_name: &str) -> Result<(), PhaseError>;
    pub fn reorder_phases(&mut self, new_order: Vec<String>) -> Result<(), PhaseError>;

    // Execution Control
    pub async fn execute_all_phases(&mut self, context: &mut PhaseContext) -> Result<(), PhaseError>;
    pub async fn execute_phase(&mut self, phase_name: &str, context: &mut PhaseContext) -> Result<(), PhaseError>;
    pub async fn execute_with_condition<F>(&mut self, condition: F, context: &mut PhaseContext) -> Result<(), PhaseError>
    where F: Fn(&PhaseContext) -> bool;

    // Configuration
    pub fn load_from_config(&mut self, config: PhaseConfigFile) -> Result<(), PhaseError>;
    pub fn save_to_config(&self) -> Result<PhaseConfigFile, PhaseError>;
}
```

### Execution Strategies

```rust
#[derive(Debug, Clone)]
pub enum ExecutionStrategy {
    Sequential,           // Execute phases one by one
    Parallel,            // Execute independent phases in parallel
    Conditional(Box<Fn(&PhaseContext) -> bool>), // Execute based on condition
    Adaptive,           // Dynamically choose strategy based on load
}

#[derive(Debug, Clone)]
pub struct ExecutionGraph {
    nodes: HashMap<String, PhaseNode>,
    edges: Vec<(String, String)>, // dependencies
}

#[derive(Debug, Clone)]
pub struct PhaseNode {
    phase: String,
    dependencies: Vec<String>,
    dependents: Vec<String>,
    execution_condition: Option<String>,
}
```

## Configuration System

### Declarative Phase Configuration

```yaml
# phase-config.yaml
phases:
  perception:
    type: "PerceptionPhase"
    enabled: true
    config:
      event_sources: ["environment", "user", "agents"]
      attention_capacity: 50
      filter_policies:
        - type: "urgency"
          threshold: 0.7
        - type: "relevance"
          threshold: 0.5

  cognition:
    type: "CognitionPhase"
    enabled: true
    dependencies: ["perception"]
    config:
      max_concurrent_goals: 5
      decision_model: "gpt-4"
      planning_horizon: 3600
      risk_tolerance: 0.3

  action:
    type: "ActionPhase"
    enabled: true
    dependencies: ["cognition"]
    config:
      parallel_execution: true
      max_concurrent_actions: 3
      timeout_ms: 5000

  reflection:
    type: "ReflectionPhase"
    enabled: true
    dependencies: ["action"]
    config:
      learning_rate: 0.1
      strategy_update_threshold: 0.05

  custom_sentiment:
    type: "CustomPhase"
    enabled: false
    dependencies: ["perception"]
    plugin: "sentiment_analyzer"
    config:
      analyzer_type: "bert_sentiment"
      output_format: "detailed"

execution_strategy: "adaptive"
error_handling:
  max_retries: 3
  retry_delay_base_ms: 1000
  fallback_enabled: true
```

### Plugin Architecture

```rust
pub trait PhasePlugin: Send + Sync {
    fn plugin_name(&self) -> &str;
    fn phase_type(&self) -> &str;
    fn version(&self) -> &str;

    fn create_phase(&self, config: &PhaseConfig) -> Result<Box<dyn Phase>, PluginError>;
    fn validate_config(&self, config: &PhaseConfig) -> Result<(), ConfigError>;
    fn get_default_config(&self) -> PhaseConfig;
}

pub struct PhaseRegistry {
    plugins: HashMap<String, Box<dyn PhasePlugin>>,
    builtin_phases: HashMap<String, Box<dyn PhaseFactory>>,
}

impl PhaseRegistry {
    pub fn register_plugin(&mut self, plugin: Box<dyn PhasePlugin>);
    pub fn create_phase(&self, phase_type: &str, config: PhaseConfig) -> Result<Box<dyn Phase>, RegistryError>;
    pub fn list_available_phases(&self) -> Vec<String>;
}
```

## Benefits of Dynamic Phase Architecture

### 1. Extensibility
- Easy addition of new cognitive capabilities
- Plugin-based phase development
- Runtime phase registration and configuration

### 2. Flexibility
- Different agent types can use different phase combinations
- Dynamic reordering of cognitive processes
- Conditional phase execution based on context

### 3. Maintainability
- Clear separation of concerns
- Independent phase testing and development
- Modular upgrade and replacement

### 4. Performance
- Optimized execution based on phase dependencies
- Parallel execution of independent phases
- Resource-aware phase scheduling

### 5. Customization
- Domain-specific phase implementations
- Configurable behavior per phase
- Adaptive strategies based on performance

This dynamic phase architecture provides a foundation for creating highly specialized, flexible, and extensible AI agents that can be tailored to specific use cases while maintaining a consistent execution framework.