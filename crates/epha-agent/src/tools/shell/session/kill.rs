//! KillSessionTool - Terminate a shell session

use std::sync::Arc;
use tokio::sync::Mutex;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::tools::shell::backend::ShellBackend;
use crate::tools::shell::error::ShellError;

/// Arguments for the KillSessionTool
#[derive(Deserialize, Debug)]
pub struct KillSessionArgs {
    /// Name of the session to kill
    pub name: String,
}

/// Output from the KillSessionTool
#[derive(Debug, Serialize)]
pub struct KillSessionOutput {
    /// Name of the killed session
    pub name: String,
    /// Whether the killed session was the current session
    pub was_current: bool,
    /// New current session (if the killed session was current)
    pub new_current: Option<String>,
}

/// Tool for killing shell sessions
pub struct KillSessionTool<B: ShellBackend> {
    backend: Arc<Mutex<B>>,
}

impl<B: ShellBackend> KillSessionTool<B> {
    /// Create a new KillSessionTool with the given backend
    pub fn new(backend: Arc<Mutex<B>>) -> Self {
        Self { backend }
    }
}

impl<B: ShellBackend + 'static> Tool for KillSessionTool<B> {
    const NAME: &'static str = "kill_session";

    type Error = ShellError;
    type Args = KillSessionArgs;
    type Output = KillSessionOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "kill_session",
            "description": "Kill a shell session and all processes running within it. \
                This permanently terminates the session. If you kill the current session, \
                the backend will automatically switch to another available session. \
                Warning: This cannot be undone.",
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the session to kill"
                    }
                },
                "required": ["name"]
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut backend = self.backend.lock().await;
        let was_current = backend.current_session() == args.name;

        // Kill the session
        backend.kill_session(&args.name).await?;

        let new_current = if was_current {
            Some(backend.current_session().to_string())
        } else {
            None
        };

        Ok(KillSessionOutput {
            name: args.name,
            was_current,
            new_current,
        })
    }
}

impl std::fmt::Display for KillSessionOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.was_current {
            if let Some(ref new) = self.new_current {
                write!(
                    f,
                    "Killed current session '{}', switched to '{}'",
                    self.name, new
                )
            } else {
                write!(
                    f,
                    "Killed session '{}' (no other sessions available)",
                    self.name
                )
            }
        } else {
            write!(f, "Killed session '{}'", self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::mock_backend::MockShellBackend;

    fn create_tool_with_mock() -> (
        KillSessionTool<MockShellBackend>,
        Arc<Mutex<MockShellBackend>>,
    ) {
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let tool = KillSessionTool::new(mock.clone());
        (tool, mock)
    }

    #[tokio::test]
    async fn test_kill_session() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("to_kill");
        }

        let result = tool
            .call(KillSessionArgs {
                name: "to_kill".into(),
            })
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.name, "to_kill");
        assert!(!output.was_current);

        // Session should be gone
        let backend = mock.lock().await;
        assert!(!backend.has_session("to_kill"));
    }

    #[tokio::test]
    async fn test_kill_current_session() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("backup");
        }

        let result = tool
            .call(KillSessionArgs {
                name: "main".into(),
            })
            .await
            .unwrap();

        assert!(result.was_current);
        // Should have switched to another session
        assert!(result.new_current.is_some());

        let backend = mock.lock().await;
        assert!(!backend.has_session("main"));
    }

    #[tokio::test]
    async fn test_kill_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let result = tool
            .call(KillSessionArgs {
                name: "nonexistent".into(),
            })
            .await;

        assert!(matches!(result, Err(ShellError::SessionNotFound { .. })));
    }

    #[tokio::test]
    async fn test_kill_then_list_consistency() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("worker");
            backend.add_session("build");
        }

        // Kill worker
        tool.call(KillSessionArgs {
            name: "worker".into(),
        })
        .await
        .unwrap();

        // Verify list_sessions reflects the change
        let backend = mock.lock().await;
        assert!(!backend.has_session("worker"));
        assert!(backend.has_session("main"));
        assert!(backend.has_session("build"));
    }

    #[test]
    fn test_kill_session_display() {
        let output = KillSessionOutput {
            name: "worker".into(),
            was_current: false,
            new_current: None,
        };
        assert!(output.to_string().contains("Killed session 'worker'"));

        let output = KillSessionOutput {
            name: "main".into(),
            was_current: true,
            new_current: Some("backup".into()),
        };
        let display = output.to_string();
        assert!(display.contains("Killed current session"));
        assert!(display.contains("switched to 'backup'"));
    }
}
