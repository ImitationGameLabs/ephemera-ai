use std::collections::HashMap;
use std::sync::Arc;

use crate::tools::AgentTool;
use llm::chat::Tool;

/// Dispatches tool calls to registered tools by name.
///
/// Replaces rig's ToolServer/ToolSet for tool routing.
pub struct ToolDispatch {
    tools: HashMap<String, Arc<dyn AgentTool>>,
}

impl ToolDispatch {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    /// Register a single tool. Panics if a tool with the same name is already registered.
    pub fn add_tool(&mut self, tool: Box<dyn AgentTool>) {
        let name = tool.name().to_string();
        assert!(
            !self.tools.contains_key(&name),
            "Duplicate tool name: '{}'",
            name
        );
        self.tools.insert(name, Arc::from(tool));
    }

    /// Register multiple tools at once.
    pub fn add_tools(&mut self, tools: Vec<Box<dyn AgentTool>>) {
        for tool in tools {
            self.add_tool(tool);
        }
    }

    /// Call a tool by name with JSON-serialized arguments.
    pub async fn call_tool(&self, name: &str, args_json: &str) -> Result<String, String> {
        let tool = self.tools.get(name).ok_or_else(|| {
            format!(
                "Unknown tool: '{}'. Available tools: {}",
                name,
                self.tools.keys().cloned().collect::<Vec<_>>().join(", ")
            )
        })?;

        tool.call(args_json)
            .await
            .map_err(|e| format!("Tool '{}' execution failed: {}", name, e))
    }

    /// Convert all registered tools to llm crate Tool structs for the API.
    #[must_use]
    pub fn to_llm_tools(&self) -> Vec<Tool> {
        self.tools.values().map(|t| t.to_llm_tool()).collect()
    }
}

impl Default for ToolDispatch {
    fn default() -> Self {
        Self::new()
    }
}
