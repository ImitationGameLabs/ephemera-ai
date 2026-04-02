//! CreateSessionTool - Create a new shell session

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::tools::AgentTool;

use crate::tools::shell::backend::ShellBackend;

/// Arguments for the CreateSessionTool
#[derive(Deserialize, Serialize, Debug)]
pub struct CreateSessionArgs {
    /// Name for the new session
    pub name: String,

    /// Working directory for the session (optional, defaults to current directory)
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

/// Output from the CreateSessionTool
#[derive(Debug, Serialize, Deserialize)]
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

#[async_trait]
impl<B: ShellBackend + Send + Sync + 'static> AgentTool for CreateSessionTool<B> {
    fn name(&self) -> &str {
        "create_session"
    }

    fn description(&self) -> &str {
        "Create a new shell session with a unique name. \
         Sessions maintain their own working directory, environment variables, \
         and command history. Use meaningful names like 'build', 'test', 'deploy' \
         to organize different types of work."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: CreateSessionArgs = serde_json::from_str(args_json)?;
        let mut backend = self.backend.lock().await;

        // Create the session
        backend
            .create_session(&args.name, args.cwd.as_deref())
            .await?;

        let cwd = args
            .cwd
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/tmp".to_string());

        let output = CreateSessionOutput { name: args.name, cwd };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::error::ShellError;
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

        let args = CreateSessionArgs { name: "worker".into(), cwd: None };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_ok());
        let output: CreateSessionOutput = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output.name, "worker");
    }

    #[tokio::test]
    async fn test_create_session_with_cwd() {
        let (tool, _) = create_tool_with_mock();

        let args =
            CreateSessionArgs { name: "worker".into(), cwd: Some(PathBuf::from("/tmp/work")) };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_ok());
        let output: CreateSessionOutput = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(output.cwd, "/tmp/work");
    }

    #[tokio::test]
    async fn test_create_duplicate_session() {
        let (tool, _) = create_tool_with_mock();

        // "main" already exists in the mock backend
        let args = CreateSessionArgs { name: "main".into(), cwd: None };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is::<ShellError>());
    }

    #[tokio::test]
    async fn test_create_multiple_sessions() {
        let (tool, mock) = create_tool_with_mock();

        let args1 = CreateSessionArgs { name: "build".into(), cwd: None };
        tool.call(&serde_json::to_string(&args1).unwrap())
            .await
            .unwrap();

        let args2 = CreateSessionArgs { name: "test".into(), cwd: None };
        tool.call(&serde_json::to_string(&args2).unwrap())
            .await
            .unwrap();

        let backend = mock.lock().await;
        assert_eq!(backend.session_count(), 3); // main + build + test
    }

    #[tokio::test]
    async fn test_create_session_with_special_name() {
        let (tool, mock) = create_tool_with_mock();

        let args = CreateSessionArgs { name: "my-test_session-123".into(), cwd: None };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_ok());
        let backend = mock.lock().await;
        assert!(backend.has_session("my-test_session-123"));
    }

    /// Test that session names containing non-ASCII characters are accepted.
    #[tokio::test]
    async fn test_create_session_with_unicode_name() {
        let (tool, mock) = create_tool_with_mock();

        let args = CreateSessionArgs { name: "测试-🌍".into(), cwd: None };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_ok());
        let backend = mock.lock().await;
        assert!(backend.has_session("测试-🌍"));
    }

    #[tokio::test]
    async fn test_create_session_name_and_schema() {
        let (tool, _) = create_tool_with_mock();
        assert_eq!(tool.name(), "create_session");
        assert!(tool.description().contains("new shell session"));
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"].is_object());
    }
}
