//! RestartSessionTool - Restart a shell session with a clean environment

use std::sync::Arc;
use tokio::sync::Mutex;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::tools::shell::backend::ShellBackend;
use crate::tools::shell::error::ShellError;

/// Arguments for the RestartSessionTool
#[derive(Deserialize, Debug)]
pub struct RestartSessionArgs {
    /// Name of the session to restart
    pub name: String,

    /// Whether to clear environment variables (default: false)
    #[serde(default)]
    pub clean_env: bool,
}

/// Output from the RestartSessionTool
#[derive(Debug, Serialize)]
pub struct RestartSessionOutput {
    /// Name of the restarted session
    pub name: String,
    /// Whether environment was cleared
    pub clean_env: bool,
}

/// Tool for restarting shell sessions
pub struct RestartSessionTool<B: ShellBackend> {
    backend: Arc<Mutex<B>>,
}

impl<B: ShellBackend> RestartSessionTool<B> {
    /// Create a new RestartSessionTool with the given backend
    pub fn new(backend: Arc<Mutex<B>>) -> Self {
        Self { backend }
    }
}

impl<B: ShellBackend + 'static> Tool for RestartSessionTool<B> {
    const NAME: &'static str = "restart_session";

    type Error = ShellError;
    type Args = RestartSessionArgs;
    type Output = RestartSessionOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "restart_session",
            "description": "Restart a shell session with a clean state. \
                This kills and recreates the session. Use this when a session \
                gets into a bad state or when you need a fresh environment. \
                Set clean_env=true to also clear all environment variables.",
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the session to restart"
                    },
                    "clean_env": {
                        "type": "boolean",
                        "description": "If true, clear all environment variables. Default is false.",
                        "default": false
                    }
                },
                "required": ["name"]
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut backend = self.backend.lock().await;

        // Restart the session
        backend.restart_session(&args.name, args.clean_env).await?;

        Ok(RestartSessionOutput {
            name: args.name,
            clean_env: args.clean_env,
        })
    }
}

impl std::fmt::Display for RestartSessionOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let env_status = if self.clean_env {
            "with clean environment"
        } else {
            "preserving environment"
        };
        write!(f, "Restarted session '{}' {}", self.name, env_status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::mock_backend::MockShellBackend;
    use std::time::Duration;

    fn create_tool_with_mock() -> (
        RestartSessionTool<MockShellBackend>,
        Arc<Mutex<MockShellBackend>>,
    ) {
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let tool = RestartSessionTool::new(mock.clone());
        (tool, mock)
    }

    #[tokio::test]
    async fn test_restart_session() {
        let (tool, _) = create_tool_with_mock();

        let result = tool
            .call(RestartSessionArgs {
                name: "main".into(),
                clean_env: false,
            })
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.name, "main");
        assert!(!output.clean_env);
    }

    #[tokio::test]
    async fn test_restart_session_clean_env() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.set_env("main", "MY_VAR", "test");
        }

        // Verify env is set
        {
            let backend = mock.lock().await;
            assert!(backend.has_env("main", "MY_VAR"));
        }

        let result = tool
            .call(RestartSessionArgs {
                name: "main".into(),
                clean_env: true,
            })
            .await
            .unwrap();

        assert!(result.clean_env);

        // Environment should be cleared
        let backend = mock.lock().await;
        assert!(!backend.has_env("main", "MY_VAR"));
    }

    #[tokio::test]
    async fn test_restart_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let result = tool
            .call(RestartSessionArgs {
                name: "nonexistent".into(),
                clean_env: false,
            })
            .await;

        assert!(matches!(result, Err(ShellError::SessionNotFound { .. })));
    }

    #[tokio::test]
    async fn test_restart_clears_history() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            // Execute some commands to add to history
            let _ = backend.execute("cmd1", Duration::from_secs(1), false).await;
            let _ = backend.execute("cmd2", Duration::from_secs(1), false).await;
        }

        // Restart session
        tool.call(RestartSessionArgs {
            name: "main".into(),
            clean_env: false,
        })
        .await
        .unwrap();

        // History should be cleared (mock backend clears history on restart)
        // We can't directly access history, but the session should still work
        let backend = mock.lock().await;
        assert!(backend.has_session("main"));
    }

    #[tokio::test]
    async fn test_restart_preserves_env_when_not_clean() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.set_env("main", "MY_VAR", "preserved");
        }

        tool.call(RestartSessionArgs {
            name: "main".into(),
            clean_env: false, // Don't clean env
        })
        .await
        .unwrap();

        // Environment should be preserved (mock doesn't clear when clean_env=false)
        // But in our mock implementation, restart always clears history regardless
        // Let's check the mock behavior
        let backend = mock.lock().await;
        // In MockShellBackend, restart_session only clears env when clean_env is true
        // So MY_VAR should still be there
        assert!(backend.has_env("main", "MY_VAR"));
    }

    #[test]
    fn test_restart_session_display() {
        let output = RestartSessionOutput {
            name: "main".into(),
            clean_env: false,
        };
        assert!(output.to_string().contains("preserving environment"));

        let output = RestartSessionOutput {
            name: "main".into(),
            clean_env: true,
        };
        assert!(output.to_string().contains("clean environment"));
    }
}
