//! CaptureOutputTool - Capture output from a session
//!
//! This tool captures recent output from a shell session,
//! useful for checking the status of background commands
//! or reviewing command output.

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::tools::AgentTool;

use super::backend::ShellBackend;

/// Default number of lines to capture
const DEFAULT_LINES: usize = 200;

/// Arguments for the CaptureOutputTool
#[derive(Deserialize, Serialize, Debug)]
pub struct CaptureOutputArgs {
    /// Target session name (uses current session if not specified)
    #[serde(default)]
    pub session: Option<String>,

    /// Maximum number of lines to capture (default: 200)
    #[serde(default)]
    pub lines: Option<usize>,
}

/// Output from the CaptureOutputTool
#[derive(Debug, Serialize, Deserialize)]
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

#[async_trait]
impl<B: ShellBackend + Send + Sync + 'static> AgentTool for CaptureOutputTool<B> {
    fn name(&self) -> &str {
        "capture_output"
    }

    fn description(&self) -> &str {
        "Capture recent output from a shell session. \
         Use this to check the status of background commands, \
         view command results, or review session history. \
         By default captures the last 200 lines."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: CaptureOutputArgs = serde_json::from_str(args_json)?;
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

        let result = CaptureOutputOutput { output, session, lines: captured_lines };

        Ok(serde_json::to_string(&result)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::error::ShellError;
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

        let args = CaptureOutputArgs { session: None, lines: None };
        let result: CaptureOutputOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
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

        let args = CaptureOutputArgs { session: None, lines: Some(10) };
        let result: CaptureOutputOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
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

        let args = CaptureOutputArgs { session: Some("worker".into()), lines: None };
        let result: CaptureOutputOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(result.session, "worker");
    }

    #[tokio::test]
    async fn test_capture_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let args = CaptureOutputArgs { session: Some("nonexistent".into()), lines: None };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is::<ShellError>());
    }

    #[tokio::test]
    async fn test_capture_empty_output() {
        let (tool, _) = create_tool_with_mock();

        let args = CaptureOutputArgs { session: None, lines: None };
        let result: CaptureOutputOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(result.session, "main");
    }

    #[tokio::test]
    async fn test_capture_zero_lines() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.set_session_output("main", vec!["line1", "line2", "line3"]);
        }

        let args = CaptureOutputArgs { session: None, lines: Some(0) };
        let result: CaptureOutputOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

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

        let args = CaptureOutputArgs { session: None, lines: Some(1000) };
        let result: CaptureOutputOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert!(result.lines <= 3);
    }

    #[tokio::test]
    async fn test_capture_name_and_schema() {
        let (tool, _) = create_tool_with_mock();
        assert_eq!(tool.name(), "capture_output");
        assert!(tool.description().contains("output"));
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["session"].is_object());
    }
}
