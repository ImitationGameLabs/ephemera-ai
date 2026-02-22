//! ListSessionsTool - List all available shell sessions

use std::sync::Arc;
use tokio::sync::Mutex;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::tools::shell::backend::{SessionInfo, ShellBackend};
use crate::tools::shell::error::ShellError;

/// Arguments for the ListSessionsTool
#[derive(Deserialize, Debug, Default)]
pub struct ListSessionsArgs;

/// Output from the ListSessionsTool
#[derive(Debug, Serialize)]
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

impl<B: ShellBackend + 'static> Tool for ListSessionsTool<B> {
    const NAME: &'static str = "list_sessions";

    type Error = ShellError;
    type Args = ListSessionsArgs;
    type Output = ListSessionsOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "list_sessions",
            "description": "List all available shell sessions. \
                Shows session names, working directories, and which session is currently focused. \
                Use this to see what sessions exist before switching or creating new ones.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let backend = self.backend.lock().await;
        let sessions = backend.list_sessions().await?;
        let current_session = backend.current_session().to_string();

        Ok(ListSessionsOutput {
            sessions,
            current_session,
        })
    }
}

impl std::fmt::Display for ListSessionsOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.sessions.is_empty() {
            write!(f, "No sessions available")?;
            return Ok(());
        }

        writeln!(f, "Shell sessions (current: {}):", self.current_session)?;
        for session in &self.sessions {
            let marker = if session.is_current { " *" } else { "" };
            writeln!(
                f,
                "  {}{} - {} ({} window{})",
                session.name,
                marker,
                session.cwd,
                session.window_count,
                if session.window_count == 1 { "" } else { "s" }
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::mock_backend::MockShellBackend;

    fn create_tool_with_mock() -> (
        ListSessionsTool<MockShellBackend>,
        Arc<Mutex<MockShellBackend>>,
    ) {
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let tool = ListSessionsTool::new(mock.clone());
        (tool, mock)
    }

    #[tokio::test]
    async fn test_list_sessions_default() {
        let (tool, _) = create_tool_with_mock();

        let result = tool.call(ListSessionsArgs::default()).await.unwrap();

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

        let result = tool.call(ListSessionsArgs::default()).await.unwrap();

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

        let _result = tool.call(ListSessionsArgs::default()).await.unwrap();

        // MockBackend will recreate or have no sessions
        // This test verifies the tool works with whatever sessions exist
    }

    #[tokio::test]
    async fn test_list_sessions_serialization() {
        let (tool, _) = create_tool_with_mock();

        let result = tool.call(ListSessionsArgs::default()).await.unwrap();

        // Verify output can be serialized to JSON
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("sessions"));
        assert!(json.contains("current_session"));
    }

    #[test]
    fn test_list_sessions_display() {
        let output = ListSessionsOutput {
            sessions: vec![
                SessionInfo {
                    name: "main".into(),
                    cwd: "/home/user".into(),
                    is_current: true,
                    window_count: 1,
                },
                SessionInfo {
                    name: "worker".into(),
                    cwd: "/tmp".into(),
                    is_current: false,
                    window_count: 2,
                },
            ],
            current_session: "main".into(),
        };

        let display = output.to_string();
        assert!(display.contains("main *"));
        assert!(display.contains("worker"));
        assert!(display.contains("2 windows"));
    }
}
