//! RestartSessionTool - Restart a shell session with a clean environment

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::tools::AgentTool;

use crate::tools::shell::backend::ShellBackend;

/// Arguments for the RestartSessionTool
#[derive(Deserialize, Serialize, Debug)]
pub struct RestartSessionArgs {
    /// Name of the session to restart
    pub name: String,

    /// Whether to clear environment variables (default: false)
    #[serde(default)]
    pub clean_env: bool,
}

/// Output from the RestartSessionTool
#[derive(Debug, Serialize, Deserialize)]
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

#[async_trait]
impl<B: ShellBackend + Send + Sync + 'static> AgentTool for RestartSessionTool<B> {
    fn name(&self) -> &str {
        "restart_session"
    }

    fn description(&self) -> &str {
        "Restart a shell session with a clean state. \
         This kills and recreates the session. Use this when a session \
         gets into a bad state or when you need a fresh environment. \
         Set clean_env=true to also clear all environment variables."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: RestartSessionArgs = serde_json::from_str(args_json)?;
        let mut backend = self.backend.lock().await;

        // Restart the session
        backend.restart_session(&args.name, args.clean_env).await?;

        let output = RestartSessionOutput { name: args.name, clean_env: args.clean_env };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::error::ShellError;
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

        let args = RestartSessionArgs { name: "main".into(), clean_env: false };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_ok());
        let output: RestartSessionOutput = serde_json::from_str(&result.unwrap()).unwrap();
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

        let args = RestartSessionArgs { name: "main".into(), clean_env: true };
        let result: RestartSessionOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert!(result.clean_env);

        // Environment should be cleared
        let backend = mock.lock().await;
        assert!(!backend.has_env("main", "MY_VAR"));
    }

    #[tokio::test]
    async fn test_restart_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let args = RestartSessionArgs { name: "nonexistent".into(), clean_env: false };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is::<ShellError>());
    }

    #[tokio::test]
    async fn test_restart_clears_history() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            let _ = backend.execute("cmd1", Duration::from_secs(1), false).await;
            let _ = backend.execute("cmd2", Duration::from_secs(1), false).await;
        }

        let args = RestartSessionArgs { name: "main".into(), clean_env: false };
        tool.call(&serde_json::to_string(&args).unwrap())
            .await
            .unwrap();

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

        let args = RestartSessionArgs { name: "main".into(), clean_env: false };
        tool.call(&serde_json::to_string(&args).unwrap())
            .await
            .unwrap();

        let backend = mock.lock().await;
        assert!(backend.has_env("main", "MY_VAR"));
    }

    #[tokio::test]
    async fn test_restart_session_name_and_schema() {
        let (tool, _) = create_tool_with_mock();
        assert_eq!(tool.name(), "restart_session");
        assert!(tool.description().contains("Restart"));
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"].is_object());
        assert!(schema["properties"]["clean_env"].is_object());
    }
}
