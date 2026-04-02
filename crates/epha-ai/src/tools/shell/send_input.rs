//! SendInputTool - Send input to a running command
//!
//! This tool sends keyboard input to a running command in a session,
//! useful for interactive commands that require user input like
//! sudo password prompts or y/n confirmations.

use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::tools::AgentTool;

use super::backend::ShellBackend;

/// Arguments for the SendInputTool
#[derive(Deserialize, Serialize, Debug)]
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
#[derive(Debug, Serialize, Deserialize)]
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

#[async_trait]
impl<B: ShellBackend + Send + Sync + 'static> AgentTool for SendInputTool<B> {
    fn name(&self) -> &str {
        "send_input"
    }

    fn description(&self) -> &str {
        "Send keyboard input to a running command in a session. \
         Use this for interactive commands that require user input, \
         such as sudo password prompts, y/n confirmations, or multi-step wizards. \
         For passwords, set press_enter to false if the prompt handles it automatically."
    }

    fn parameters_schema(&self) -> Value {
        json!({
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
        })
    }

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: SendInputArgs = serde_json::from_str(args_json)?;
        let mut backend = self.backend.lock().await;

        // Switch to specified session if provided
        if let Some(ref session) = args.session {
            backend.switch_session(session).await?;
        }

        let session = backend.current_session().to_string();

        // Send the input
        backend.send_input(&args.input, args.press_enter).await?;

        let output = SendInputOutput { input: args.input, press_enter: args.press_enter, session };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::error::ShellError;
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

        let args = SendInputArgs { input: "y".into(), session: None, press_enter: true };
        let result: SendInputOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
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

        let args =
            SendInputArgs { input: "secret_password".into(), session: None, press_enter: false };
        let result: SendInputOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert!(!result.press_enter);

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], "secret_password");
    }

    #[tokio::test]
    async fn test_send_input_multiline() {
        let (tool, mock) = create_tool_with_mock();

        let args =
            SendInputArgs { input: "line1\nline2\nline3".into(), session: None, press_enter: true };
        tool.call(&serde_json::to_string(&args).unwrap())
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

        let args = SendInputArgs {
            input: "test".into(),
            session: Some("worker".into()),
            press_enter: true,
        };
        let result: SendInputOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(result.session, "worker");
    }

    #[tokio::test]
    async fn test_send_input_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let args = SendInputArgs {
            input: "test".into(),
            session: Some("nonexistent".into()),
            press_enter: true,
        };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is::<ShellError>());
    }

    #[tokio::test]
    async fn test_send_input_empty() {
        let (tool, mock) = create_tool_with_mock();

        let args = SendInputArgs { input: "".into(), session: None, press_enter: true };
        tool.call(&serde_json::to_string(&args).unwrap())
            .await
            .unwrap();

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], "\n");
    }

    /// Test that input containing non-ASCII characters is passed through correctly.
    #[tokio::test]
    async fn test_send_input_unicode() {
        let (tool, mock) = create_tool_with_mock();

        let args =
            SendInputArgs { input: "你好世界 🌍".into(), session: None, press_enter: false };
        tool.call(&serde_json::to_string(&args).unwrap())
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

        let args = SendInputArgs { input: long_input.clone(), session: None, press_enter: true };
        tool.call(&serde_json::to_string(&args).unwrap())
            .await
            .unwrap();

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], expected);
    }

    #[tokio::test]
    async fn test_send_input_control_characters() {
        let (tool, mock) = create_tool_with_mock();

        let args = SendInputArgs { input: "\x03".into(), session: None, press_enter: false };
        tool.call(&serde_json::to_string(&args).unwrap())
            .await
            .unwrap();

        let backend = mock.lock().await;
        let inputs = backend.get_inputs_received();
        assert_eq!(inputs[0], "\x03");
    }

    #[tokio::test]
    async fn test_send_input_name_and_schema() {
        let (tool, _) = create_tool_with_mock();
        assert_eq!(tool.name(), "send_input");
        assert!(tool.description().contains("keyboard input"));
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["input"].is_object());
    }
}
