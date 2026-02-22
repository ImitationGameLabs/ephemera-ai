//! SwitchSessionTool - Switch to a different shell session

use std::sync::Arc;
use tokio::sync::Mutex;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::tools::shell::backend::ShellBackend;
use crate::tools::shell::error::ShellError;

/// Arguments for the SwitchSessionTool
#[derive(Deserialize, Debug)]
pub struct SwitchSessionArgs {
    /// Name of the session to switch to
    pub name: String,
}

/// Output from the SwitchSessionTool
#[derive(Debug, Serialize)]
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

impl<B: ShellBackend + 'static> Tool for SwitchSessionTool<B> {
    const NAME: &'static str = "switch_session";

    type Error = ShellError;
    type Args = SwitchSessionArgs;
    type Output = SwitchSessionOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "switch_session",
            "description": "Switch to a different shell session. \
                After switching, all subsequent commands will run in the target session. \
                Use list_sessions first to see available sessions.",
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the session to switch to"
                    }
                },
                "required": ["name"]
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut backend = self.backend.lock().await;
        let previous_session = backend.current_session().to_string();

        // Switch to the session
        backend.switch_session(&args.name).await?;

        Ok(SwitchSessionOutput {
            name: args.name,
            previous_session,
        })
    }
}

impl std::fmt::Display for SwitchSessionOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Switched from '{}' to '{}'",
            self.previous_session, self.name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::mock_backend::MockShellBackend;

    fn create_tool_with_mock() -> (
        SwitchSessionTool<MockShellBackend>,
        Arc<Mutex<MockShellBackend>>,
    ) {
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

        let result = tool
            .call(SwitchSessionArgs {
                name: "worker".into(),
            })
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.name, "worker");
        assert_eq!(output.previous_session, "main");
    }

    #[tokio::test]
    async fn test_switch_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let result = tool
            .call(SwitchSessionArgs {
                name: "nonexistent".into(),
            })
            .await;

        assert!(matches!(result, Err(ShellError::SessionNotFound { .. })));
    }

    #[tokio::test]
    async fn test_switch_to_same_session() {
        let (tool, mock) = create_tool_with_mock();

        let result = tool
            .call(SwitchSessionArgs {
                name: "main".into(),
            })
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.name, "main");
        assert_eq!(output.previous_session, "main");

        // Session should still be main
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

        // Switch to worker
        tool.call(SwitchSessionArgs {
            name: "worker".into(),
        })
        .await
        .unwrap();

        // Verify current session is worker
        let backend = mock.lock().await;
        assert_eq!(backend.current_session(), "worker");
    }

    #[test]
    fn test_switch_session_display() {
        let output = SwitchSessionOutput {
            name: "worker".into(),
            previous_session: "main".into(),
        };
        let display = output.to_string();
        assert!(display.contains("main"));
        assert!(display.contains("worker"));
    }
}
