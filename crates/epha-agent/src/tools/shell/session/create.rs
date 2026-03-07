//! CreateSessionTool - Create a new shell session

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::tools::shell::backend::ShellBackend;
use crate::tools::shell::error::ShellError;

/// Arguments for the CreateSessionTool
#[derive(Deserialize, Debug)]
pub struct CreateSessionArgs {
    /// Name for the new session
    pub name: String,

    /// Working directory for the session (optional, defaults to current directory)
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

/// Output from the CreateSessionTool
#[derive(Debug, Serialize)]
pub struct CreateSessionOutput {
    /// Name of the created session
    pub name: String,
    /// Working directory of the session
    pub cwd: String,
}

/// Tool for creating new shell sessions
pub struct CreateSessionTool<B: ShellBackend> {
    backend: Arc<Mutex<B>>,
}

impl<B: ShellBackend> CreateSessionTool<B> {
    /// Create a new CreateSessionTool with the given backend
    pub fn new(backend: Arc<Mutex<B>>) -> Self {
        Self { backend }
    }
}

impl<B: ShellBackend + 'static> Tool for CreateSessionTool<B> {
    const NAME: &'static str = "create_session";

    type Error = ShellError;
    type Args = CreateSessionArgs;
    type Output = CreateSessionOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "create_session",
            "description": "Create a new shell session with a unique name. \
                Sessions maintain their own working directory, environment variables, \
                and command history. Use meaningful names like 'build', 'test', 'deploy' \
                to organize different types of work.",
            "parameters": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Unique name for the new session. Use descriptive names like 'build', 'test', 'deploy'."
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory for the session. Defaults to the current directory if not specified."
                    }
                },
                "required": ["name"]
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut backend = self.backend.lock().await;

        // Create the session
        backend
            .create_session(&args.name, args.cwd.as_deref())
            .await?;

        let cwd = args
            .cwd
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/tmp".to_string());

        Ok(CreateSessionOutput {
            name: args.name,
            cwd,
        })
    }
}

impl std::fmt::Display for CreateSessionOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Created session '{}' in directory '{}'",
            self.name, self.cwd
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::mock_backend::MockShellBackend;

    fn create_tool_with_mock() -> (
        CreateSessionTool<MockShellBackend>,
        Arc<Mutex<MockShellBackend>>,
    ) {
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let tool = CreateSessionTool::new(mock.clone());
        (tool, mock)
    }

    #[tokio::test]
    async fn test_create_session() {
        let (tool, _) = create_tool_with_mock();

        let result = tool
            .call(CreateSessionArgs {
                name: "worker".into(),
                cwd: None,
            })
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.name, "worker");
    }

    #[tokio::test]
    async fn test_create_session_with_cwd() {
        let (tool, _) = create_tool_with_mock();

        let result = tool
            .call(CreateSessionArgs {
                name: "worker".into(),
                cwd: Some(PathBuf::from("/tmp/work")),
            })
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.cwd, "/tmp/work");
    }

    #[tokio::test]
    async fn test_create_duplicate_session() {
        let (tool, _) = create_tool_with_mock();

        // "main" already exists in the mock backend
        let result = tool
            .call(CreateSessionArgs {
                name: "main".into(),
                cwd: None,
            })
            .await;

        assert!(matches!(result, Err(ShellError::SessionExists { .. })));
    }

    #[tokio::test]
    async fn test_create_multiple_sessions() {
        let (tool, mock) = create_tool_with_mock();

        tool.call(CreateSessionArgs {
            name: "build".into(),
            cwd: None,
        })
        .await
        .unwrap();

        tool.call(CreateSessionArgs {
            name: "test".into(),
            cwd: None,
        })
        .await
        .unwrap();

        let backend = mock.lock().await;
        assert_eq!(backend.session_count(), 3); // main + build + test
    }

    #[tokio::test]
    async fn test_create_session_with_special_name() {
        let (tool, mock) = create_tool_with_mock();

        // Create session with dashes and underscores
        let result = tool
            .call(CreateSessionArgs {
                name: "my-test_session-123".into(),
                cwd: None,
            })
            .await;

        assert!(result.is_ok());
        let backend = mock.lock().await;
        assert!(backend.has_session("my-test_session-123"));
    }

    #[tokio::test]
    async fn test_create_session_with_unicode_name() {
        let (tool, mock) = create_tool_with_mock();

        // Create session with unicode name
        let result = tool
            .call(CreateSessionArgs {
                name: "测试-🌍".into(),
                cwd: None,
            })
            .await;

        // This should work with mock backend
        assert!(result.is_ok());
        let backend = mock.lock().await;
        assert!(backend.has_session("测试-🌍"));
    }

    #[test]
    fn test_create_session_display() {
        let output = CreateSessionOutput {
            name: "worker".into(),
            cwd: "/home/user/project".into(),
        };
        assert!(output.to_string().contains("worker"));
        assert!(output.to_string().contains("/home/user/project"));
    }
}
