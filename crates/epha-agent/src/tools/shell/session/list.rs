//! ListSessionsTool - List all available shell sessions

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::tools::AgentTool;

use crate::tools::shell::backend::{SessionInfo, ShellBackend};

/// Arguments for the ListSessionsTool
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ListSessionsArgs;

/// Output from the ListSessionsTool
#[derive(Debug, Serialize, Deserialize)]
pub struct ListSessionsOutput {
    /// List of available sessions
    pub sessions: Vec<SessionInfo>,
    /// Name of the currently focused session
    pub current_session: String,
}

/// Tool for listing all shell sessions
pub struct ListSessionsTool<B: ShellBackend> {
    backend: Arc<Mutex<B>>,
}

impl<B: ShellBackend> ListSessionsTool<B> {
    /// Create a new ListSessionsTool with the given backend
    pub fn new(backend: Arc<Mutex<B>>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl<B: ShellBackend + Send + Sync + 'static> AgentTool for ListSessionsTool<B> {
    fn name(&self) -> &str {
        "list_sessions"
    }

    fn description(&self) -> &str {
        "List all available shell sessions. \
         Shows session names, working directories, and which session is currently focused. \
         Use this to see what sessions exist before switching or creating new ones."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let _args: ListSessionsArgs = serde_json::from_str(args_json)?;
        let backend = self.backend.lock().await;
        let sessions = backend.list_sessions().await?;
        let current_session = backend.current_session().to_string();

        let output = ListSessionsOutput { sessions, current_session };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::mock_backend::MockShellBackend;

    fn create_tool_with_mock() -> (ListSessionsTool<MockShellBackend>, Arc<Mutex<MockShellBackend>>)
    {
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let tool = ListSessionsTool::new(mock.clone());
        (tool, mock)
    }

    #[tokio::test]
    async fn test_list_sessions_default() {
        let (tool, _) = create_tool_with_mock();

        let args = ListSessionsArgs::default();
        let result: ListSessionsOutput =
            serde_json::from_str(&tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap())
                .unwrap();

        assert_eq!(result.sessions.len(), 1);
        assert_eq!(result.current_session, "main");
        assert!(result.sessions[0].is_current);
    }

    #[tokio::test]
    async fn test_list_sessions_multiple() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("build");
            backend.add_session("test");
        }

        let args = ListSessionsArgs::default();
        let result: ListSessionsOutput =
            serde_json::from_str(&tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap())
                .unwrap();

        assert_eq!(result.sessions.len(), 3);
        assert_eq!(result.current_session, "main");
    }

    #[tokio::test]
    async fn test_list_sessions_empty() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            // Kill the default session
            backend.kill_session("main").await.ok();
        }

        let args = ListSessionsArgs::default();
        let _result = tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();
    }

    #[tokio::test]
    async fn test_list_sessions_serialization() {
        let (tool, _) = create_tool_with_mock();

        let args = ListSessionsArgs::default();
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();

        // Verify output is valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["sessions"].is_array());
        assert!(parsed["current_session"].is_string());
    }

    #[tokio::test]
    async fn test_list_sessions_name_and_schema() {
        let (tool, _) = create_tool_with_mock();
        assert_eq!(tool.name(), "list_sessions");
        assert!(tool.description().contains("List"));
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"], serde_json::json!({}));
    }
}
