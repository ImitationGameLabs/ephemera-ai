//! CaptureOutputTool - Capture output from a session
//!
//! This tool captures recent output from a shell session,
//! useful for checking the status of background commands
//! or reviewing command output.

use std::sync::Arc;
use tokio::sync::Mutex;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::backend::ShellBackend;
use super::error::ShellError;

/// Default number of lines to capture
const DEFAULT_LINES: usize = 200;

/// Arguments for the CaptureOutputTool
#[derive(Deserialize, Debug)]
pub struct CaptureOutputArgs {
    /// Target session name (uses current session if not specified)
    #[serde(default)]
    pub session: Option<String>,

    /// Maximum number of lines to capture (default: 200)
    #[serde(default)]
    pub lines: Option<usize>,
}

/// Output from the CaptureOutputTool
#[derive(Debug, Serialize)]
pub struct CaptureOutputOutput {
    /// The captured output
    pub output: String,
    /// Session name where output was captured
    pub session: String,
    /// Number of lines captured
    pub lines: usize,
}

/// Tool for capturing output from a session
pub struct CaptureOutputTool<B: ShellBackend> {
    backend: Arc<Mutex<B>>,
}

impl<B: ShellBackend> CaptureOutputTool<B> {
    /// Create a new CaptureOutputTool with the given backend
    pub fn new(backend: Arc<Mutex<B>>) -> Self {
        Self { backend }
    }
}

impl<B: ShellBackend + 'static> Tool for CaptureOutputTool<B> {
    const NAME: &'static str = "capture_output";

    type Error = ShellError;
    type Args = CaptureOutputArgs;
    type Output = CaptureOutputOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "capture_output",
            "description": "Capture recent output from a shell session. \
                Use this to check the status of background commands, \
                view command results, or review session history. \
                By default captures the last 200 lines.",
            "parameters": {
                "type": "object",
                "properties": {
                    "session": {
                        "type": "string",
                        "description": "Target session name. Uses the current focused session if not specified."
                    },
                    "lines": {
                        "type": "integer",
                        "description": "Maximum number of lines to capture. Default is 200.",
                        "default": 200
                    }
                },
                "required": []
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut backend = self.backend.lock().await;

        // Switch to specified session if provided
        if let Some(ref session) = args.session {
            backend.switch_session(session).await?;
        }

        let session = backend.current_session().to_string();
        let lines = args.lines.unwrap_or(DEFAULT_LINES);

        // Capture the output
        let output = backend.capture_output(lines).await?;
        let captured_lines = output.lines().count();

        Ok(CaptureOutputOutput {
            output,
            session,
            lines: captured_lines,
        })
    }
}

impl std::fmt::Display for CaptureOutputOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Session '{}' ({} lines):\n{}",
            self.session, self.lines, self.output
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::mock_backend::MockShellBackend;

    fn create_tool_with_mock() -> (
        CaptureOutputTool<MockShellBackend>,
        Arc<Mutex<MockShellBackend>>,
    ) {
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let tool = CaptureOutputTool::new(mock.clone());
        (tool, mock)
    }

    #[tokio::test]
    async fn test_capture_default_lines() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.set_session_output("main", vec!["line1", "line2", "line3"]);
        }

        let result = tool
            .call(CaptureOutputArgs {
                session: None,
                lines: None,
            })
            .await
            .unwrap();

        assert_eq!(result.session, "main");
        assert!(result.output.contains("line3"));
    }

    #[tokio::test]
    async fn test_capture_limited_lines() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            let lines: Vec<&str> = (1..=100)
                .map(|i| Box::leak(format!("line {}", i).into_boxed_str()) as &str)
                .collect();
            backend.set_session_output("main", lines);
        }

        let result = tool
            .call(CaptureOutputArgs {
                session: None,
                lines: Some(10),
            })
            .await
            .unwrap();

        // Mock backend returns lines in reverse order with take, so it takes the last 10
        assert!(result.lines <= 10);
    }

    #[tokio::test]
    async fn test_capture_specific_session() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("worker");
            backend.set_session_output("worker", vec!["worker output"]);
        }

        let result = tool
            .call(CaptureOutputArgs {
                session: Some("worker".into()),
                lines: None,
            })
            .await
            .unwrap();

        assert_eq!(result.session, "worker");
    }

    #[tokio::test]
    async fn test_capture_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let result = tool
            .call(CaptureOutputArgs {
                session: Some("nonexistent".into()),
                lines: None,
            })
            .await;

        assert!(matches!(result, Err(ShellError::SessionNotFound { .. })));
    }

    #[tokio::test]
    async fn test_capture_empty_output() {
        let (tool, _) = create_tool_with_mock();

        let result = tool
            .call(CaptureOutputArgs {
                session: None,
                lines: None,
            })
            .await
            .unwrap();

        // Empty output is valid
        assert_eq!(result.session, "main");
    }

    #[tokio::test]
    async fn test_capture_zero_lines() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.set_session_output("main", vec!["line1", "line2", "line3"]);
        }

        let result = tool
            .call(CaptureOutputArgs {
                session: None,
                lines: Some(0),
            })
            .await
            .unwrap();

        // Zero lines should return empty output
        assert_eq!(result.output, "");
        assert_eq!(result.lines, 0);
    }

    #[tokio::test]
    async fn test_capture_more_lines_than_available() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.set_session_output("main", vec!["line1", "line2", "line3"]);
        }

        let result = tool
            .call(CaptureOutputArgs {
                session: None,
                lines: Some(1000), // Request more than available
            })
            .await
            .unwrap();

        // Should return all available lines
        assert!(result.lines <= 3);
    }

    #[test]
    fn test_capture_output_display() {
        let output = CaptureOutputOutput {
            output: "line1\nline2".into(),
            session: "main".into(),
            lines: 2,
        };
        let display = output.to_string();
        assert!(display.contains("main"));
        assert!(display.contains("2 lines"));
        assert!(display.contains("line1"));
    }
}
