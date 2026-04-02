//! KillSessionTool - Terminate a shell session

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::tools::AgentTool;

use crate::tools::shell::backend::ShellBackend;

/// Arguments for the KillSessionTool
#[derive(Deserialize, Serialize, Debug)]
pub struct KillSessionArgs {
    /// Name of the session to kill
    pub name: String,
}

/// Output from the KillSessionTool
#[derive(Debug, Serialize, Deserialize)]
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

#[async_trait]
impl<B: ShellBackend + Send + Sync + 'static> AgentTool for KillSessionTool<B> {
    fn name(&self) -> &str {
        "kill_session"
    }

    fn description(&self) -> &str {
        "Kill a shell session and all processes running within it. \
         This permanently terminates the session. If you kill the current session, \
         the backend will automatically switch to another available session. \
         Warning: This cannot be undone."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the session to kill"
                }
            },
            "required": ["name"]
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: KillSessionArgs = serde_json::from_str(args_json)?;
        let mut backend = self.backend.lock().await;
        let was_current = backend.current_session() == args.name;

        // Kill the session
        backend.kill_session(&args.name).await?;

        let new_current =
            if was_current { Some(backend.current_session().to_string()) } else { None };

        let output = KillSessionOutput { name: args.name, was_current, new_current };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::error::ShellError;
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

        let args = KillSessionArgs { name: "to_kill".into() };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_ok());
        let output: KillSessionOutput = serde_json::from_str(&result.unwrap()).unwrap();
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

        let args = KillSessionArgs { name: "main".into() };
        let result: KillSessionOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert!(result.was_current);
        assert!(result.new_current.is_some());

        let backend = mock.lock().await;
        assert!(!backend.has_session("main"));
    }

    #[tokio::test]
    async fn test_kill_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let args = KillSessionArgs { name: "nonexistent".into() };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is::<ShellError>());
    }

    #[tokio::test]
    async fn test_kill_then_list_consistency() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("worker");
            backend.add_session("build");
        }

        let args = KillSessionArgs { name: "worker".into() };
        tool.call(&serde_json::to_string(&args).unwrap())
            .await
            .unwrap();

        let backend = mock.lock().await;
        assert!(!backend.has_session("worker"));
        assert!(backend.has_session("main"));
        assert!(backend.has_session("build"));
    }

    #[tokio::test]
    async fn test_kill_session_name_and_schema() {
        let (tool, _) = create_tool_with_mock();
        assert_eq!(tool.name(), "kill_session");
        assert!(tool.description().contains("Kill"));
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["name"].is_object());
    }
}
