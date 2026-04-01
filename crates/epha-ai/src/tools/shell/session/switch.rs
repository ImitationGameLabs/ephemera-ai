//! SwitchSessionTool - Switch to a different shell session

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::tools::AgentTool;

use crate::tools::shell::backend::ShellBackend;

/// Arguments for the SwitchSessionTool
#[derive(Deserialize, Serialize, Debug)]
pub struct SwitchSessionArgs {
    /// Name of the session to switch to
    pub name: String,
}

/// Output from the SwitchSessionTool
#[derive(Debug, Serialize, Deserialize)]
pub struct SwitchSessionOutput {
    /// Name of the session switched to
    pub name: String,
    /// Previous session name
    pub previous_session: String,
}

/// Tool for switching between shell sessions
pub struct SwitchSessionTool<B: ShellBackend> {
    backend: Arc<Mutex<B>>,
}

impl<B: ShellBackend> SwitchSessionTool<B> {
    /// Create a new SwitchSessionTool with the given backend
    pub fn new(backend: Arc<Mutex<B>>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl<B: ShellBackend + Send + Sync + 'static> AgentTool for SwitchSessionTool<B> {
    fn name(&self) -> &str {
        "switch_session"
    }

    fn description(&self) -> &str {
        "Switch to a different shell session. \
         After switching, all subsequent commands will run in the target session. \
         Use list_sessions first to see available sessions."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the session to switch to"
                }
            },
            "required": ["name"]
        })
    }

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: SwitchSessionArgs = serde_json::from_str(args_json)?;
        let mut backend = self.backend.lock().await;
        let previous_session = backend.current_session().to_string();

        // Switch to the session
        backend.switch_session(&args.name).await?;

        let output = SwitchSessionOutput { name: args.name, previous_session };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::error::ShellError;
    use crate::tools::shell::mock_backend::MockShellBackend;

    fn create_tool_with_mock() -> (SwitchSessionTool<MockShellBackend>, Arc<Mutex<MockShellBackend>>)
    {
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let tool = SwitchSessionTool::new(mock.clone());
        (tool, mock)
    }

    #[tokio::test]
    async fn test_switch_session() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("worker");
        }

        let args = SwitchSessionArgs { name: "worker".into() };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_ok());
        let output: SwitchSessionOutput = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output.name, "worker");
        assert_eq!(output.previous_session, "main");
    }

    #[tokio::test]
    async fn test_switch_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let args = SwitchSessionArgs { name: "nonexistent".into() };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is::<ShellError>());
    }

    #[tokio::test]
    async fn test_switch_to_same_session() {
        let (tool, mock) = create_tool_with_mock();

        let args = SwitchSessionArgs { name: "main".into() };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_ok());
        let output: SwitchSessionOutput = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output.name, "main");
        assert_eq!(output.previous_session, "main");

        let backend = mock.lock().await;
        assert_eq!(backend.current_session(), "main");
    }

    #[tokio::test]
    async fn test_switch_then_execute_command() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("worker");
            backend.push_output("worker output");
            backend.push_exit_code(0);
        }

        let args = SwitchSessionArgs { name: "worker".into() };
        tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();

        let backend = mock.lock().await;
        assert_eq!(backend.current_session(), "worker");
    }

    #[tokio::test]
    async fn test_switch_session_name_and_schema() {
        let (tool, _) = create_tool_with_mock();
        assert_eq!(tool.name(), "switch_session");
        assert!(tool.description().contains("Switch"));
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"].is_object());
    }
}
