use async_trait::async_trait;
use llm::chat::{FunctionTool, Tool};
use serde_json::Value;

/// Custom tool trait for agent tools, replacing rig::tool::Tool.
///
/// This trait is object-safe and uses string-based JSON for args/results,
/// eliminating rig's type-parameterized approach while keeping the core
/// execution logic.
#[async_trait]
pub trait AgentTool: Send + Sync {
    /// Returns the tool name used in LLM function calls.
    fn name(&self) -> &str;

    /// Returns a human-readable description for the LLM.
    fn description(&self) -> &str;

    /// Returns the JSON Schema for the tool's parameters.
    fn parameters_schema(&self) -> Value;

    /// Convert to the llm crate's Tool struct for the LLM API.
    #[must_use]
    fn to_llm_tool(&self) -> Tool {
        Tool {
            tool_type: "function".to_string(),
            function: FunctionTool {
                name: self.name().to_string(),
                description: self.description().to_string(),
                parameters: self.parameters_schema(),
            },
        }
    }

    /// Execute the tool with JSON-serialized arguments, returning a result string.
    ///
    /// Error semantics:
    /// - `Ok(String)` — tool executed normally; the string may indicate a
    ///   business-logic failure (e.g. "shell command exited with non-zero code").
    ///
    /// - `Err` — system-level abnormality (network timeout, serialization
    ///   failure, programming bug). Logged at error level and may trigger
    ///   operator alerts.
    async fn call(&self, args_json: &str) -> anyhow::Result<String>;
}
