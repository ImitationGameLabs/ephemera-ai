//! SendInputTool - Send input to a running command
//!
//! This tool sends keyboard input to a running command in a session,
//! useful for interactive commands that require user input like
//! sudo password prompts or y/n confirmations.

use std::sync::Arc;
use tokio::sync::Mutex;

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::backend::ShellBackend;
use super::error::ShellError;

/// Arguments for the SendInputTool
#[derive(Deserialize, Debug)]
pub struct SendInputArgs {
    /// The input text to send
    pub input: String,

    /// Target session name (uses current session if not specified)
    #[serde(default)]
    pub session: Option<String>,

    /// Whether to press Enter after sending the input (default: true)
    #[serde(default = "default_press_enter")]
    pub press_enter: bool,
}

fn default_press_enter() -> bool {
    true
}

/// Output from the SendInputTool
#[derive(Debug, Serialize)]
pub struct SendInputOutput {
    /// The input that was sent
    pub input: String,
    /// Whether Enter was pressed
    pub press_enter: bool,
    /// Session name where input was sent
    pub session: String,
}

/// Tool for sending input to running commands
pub struct SendInputTool<B: ShellBackend> {
    backend: Arc<Mutex<B>>,
}

impl<B: ShellBackend> SendInputTool<B> {
    /// Create a new SendInputTool with the given backend
    pub fn new(backend: Arc<Mutex<B>>) -> Self {
        Self { backend }
    }
}

impl<B: ShellBackend + 'static> Tool for SendInputTool<B> {
    const NAME: &'static str = "send_input";

    type Error = ShellError;
    type Args = SendInputArgs;
    type Output = SendInputOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "send_input",
            "description": "Send keyboard input to a running command in a session. \
                Use this for interactive commands that require user input, \
                such as sudo password prompts, y/n confirmations, or multi-step wizards. \
                For passwords, set press_enter to false if the prompt handles it automatically.",
            "parameters": {
                "type": "object",
                "properties": {
                    "input": {
                        "type": "string",
                        "description": "The input text to send to the running command"
                    },
                    "session": {
                        "type": "string",
                        "description": "Target session name. Uses the current focused session if not specified."
                    },
                    "press_enter": {
                        "type": "boolean",
                        "description": "Whether to press Enter after sending the input. Default is true. Set to false for password prompts.",
                        "default": true
                    }
                },
                "required": ["input"]
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

        // Send the input
        backend.send_input(&args.input, args.press_enter).await?;

        Ok(SendInputOutput {
            input: args.input,
            press_enter: args.press_enter,
            session,
        })
    }
}

impl std::fmt::Display for SendInputOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enter_suffix = if self.press_enter { " (Enter)" } else { "" };
        write!(
            f,
            "Sent input '{}' to session '{}'{}",
            self.input, self.session, enter_suffix
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::mock_backend::MockShellBackend;

    fn create_tool_with_mock() -> (
        SendInputTool<MockShellBackend>,
        Arc<Mutex<MockShellBackend>>,
    ) {
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let tool = SendInputTool::new(mock.clone());
        (tool, mock)
    }

    #[tokio::test]
    async fn test_send_input_basic() {
        let (tool, mock) = create_tool_with_mock();

        let result = tool
            .call(SendInputArgs {
                input: "y".into(),
                session: None,
                press_enter: true,
            })
            .await
            .unwrap();

        assert_eq!(result.input, "y");
        assert!(result.press_enter);
        assert_eq!(result.session, "main");

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0], "y\n");
    }

    #[tokio::test]
    async fn test_send_input_without_enter() {
        let (tool, mock) = create_tool_with_mock();

        let result = tool
            .call(SendInputArgs {
                input: "secret_password".into(),
                session: None,
                press_enter: false,
            })
            .await
            .unwrap();

        assert!(!result.press_enter);

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], "secret_password"); // No newline
    }

    #[tokio::test]
    async fn test_send_input_multiline() {
        let (tool, mock) = create_tool_with_mock();

        tool.call(SendInputArgs {
            input: "line1\nline2\nline3".into(),
            session: None,
            press_enter: true,
        })
        .await
        .unwrap();

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], "line1\nline2\nline3\n");
    }

    #[tokio::test]
    async fn test_send_input_to_specific_session() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("worker");
        }

        let result = tool
            .call(SendInputArgs {
                input: "test".into(),
                session: Some("worker".into()),
                press_enter: true,
            })
            .await
            .unwrap();

        assert_eq!(result.session, "worker");
    }

    #[tokio::test]
    async fn test_send_input_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let result = tool
            .call(SendInputArgs {
                input: "test".into(),
                session: Some("nonexistent".into()),
                press_enter: true,
            })
            .await;

        assert!(matches!(result, Err(ShellError::SessionNotFound { .. })));
    }

    #[tokio::test]
    async fn test_send_input_empty() {
        let (tool, mock) = create_tool_with_mock();

        tool.call(SendInputArgs {
            input: "".into(),
            session: None,
            press_enter: true,
        })
        .await
        .unwrap();

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], "\n"); // Just the newline
    }

    #[tokio::test]
    async fn test_send_input_unicode() {
        let (tool, mock) = create_tool_with_mock();

        tool.call(SendInputArgs {
            input: "你好世界 🌍".into(),
            session: None,
            press_enter: false,
        })
        .await
        .unwrap();

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], "你好世界 🌍");
    }

    #[tokio::test]
    async fn test_send_input_long_input() {
        let (tool, mock) = create_tool_with_mock();

        let long_input = "x".repeat(10000);
        let expected = format!("{}\n", long_input);

        tool.call(SendInputArgs {
            input: long_input.clone(),
            session: None,
            press_enter: true,
        })
        .await
        .unwrap();

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], expected);
    }

    #[tokio::test]
    async fn test_send_input_control_characters() {
        let (tool, mock) = create_tool_with_mock();

        // Test sending control characters (like Ctrl-C which is \x03)
        tool.call(SendInputArgs {
            input: "\x03".into(), // Ctrl-C
            session: None,
            press_enter: false,
        })
        .await
        .unwrap();

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], "\x03");
    }

    #[test]
    fn test_send_input_output_display() {
        let output = SendInputOutput {
            input: "y".into(),
            press_enter: true,
            session: "main".into(),
        };
        assert!(output.to_string().contains("y"));
        assert!(output.to_string().contains("Enter"));

        let output = SendInputOutput {
            input: "password".into(),
            press_enter: false,
            session: "main".into(),
        };
        assert!(!output.to_string().contains("Enter"));
    }
}
