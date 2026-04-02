//! BashTool - Execute shell commands in a session
//!
//! This tool executes commands in a shell session, supporting both
//! synchronous (wait for completion) and background (fire-and-forget) modes.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::tools::AgentTool;

use super::backend::ShellBackend;

/// Default timeout for command execution (seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Arguments for the BashTool
#[derive(Deserialize, Serialize, Debug)]
pub struct BashArgs {
    /// The command to execute
    pub command: String,

    /// Target session name (uses current session if not specified)
    #[serde(default)]
    pub session: Option<String>,

    /// Timeout in seconds (default: 120)
    #[serde(default)]
    pub timeout: Option<u64>,

    /// Run in background mode (don't wait for completion)
    #[serde(default)]
    pub background: bool,
}

/// Output from the BashTool
#[derive(Debug, Serialize, Deserialize)]
pub struct BashOutput {
    /// Command output (stdout and stderr combined)
    pub output: String,
    /// Exit code (None if timed out or background mode)
    pub exit_code: Option<i32>,
    /// Whether the command timed out
    pub timed_out: bool,
    /// Session name where the command was executed
    pub session: String,
}

/// Tool for executing shell commands
pub struct BashTool<B: ShellBackend> {
    backend: Arc<Mutex<B>>,
}

impl<B: ShellBackend> BashTool<B> {
    /// Create a new BashTool with the given backend
    pub fn new(backend: Arc<Mutex<B>>) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl<B: ShellBackend + Send + Sync + 'static> AgentTool for BashTool<B> {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a shell command in a persistent session. \
         Supports timeout and background execution. \
         Commands run in a tmux session, so environment variables, \
         working directory, and command history persist across calls."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "session": {
                    "type": "string",
                    "description": "Target session name. Uses the current focused session if not specified."
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds. Default is 120 seconds.",
                    "default": 120
                },
                "background": {
                    "type": "boolean",
                    "description": "Run in background mode. Returns immediately without waiting for command completion.",
                    "default": false
                }
            },
            "required": ["command"]
        })
    }

    async fn call(&self, args_json: &str) -> anyhow::Result<String> {
        let args: BashArgs = serde_json::from_str(args_json)?;
        let timeout = Duration::from_secs(args.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS));

        let mut backend = self.backend.lock().await;

        // Switch to specified session if provided
        if let Some(ref session) = args.session {
            backend.switch_session(session).await?;
        }

        let session = backend.current_session().to_string();

        // Execute the command
        let result = backend
            .execute(&args.command, timeout, args.background)
            .await?;

        let output = BashOutput {
            output: result.output,
            exit_code: result.exit_code,
            timed_out: result.timed_out,
            session,
        };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::shell::error::ShellError;
    use crate::tools::shell::mock_backend::MockShellBackend;

    fn create_tool_with_mock() -> (BashTool<MockShellBackend>, Arc<Mutex<MockShellBackend>>) {
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let tool = BashTool::new(mock.clone());
        (tool, mock)
    }

    #[tokio::test]
    async fn test_bash_simple_command() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.push_output("hello world");
            backend.push_exit_code(0);
        }

        let args = BashArgs {
            command: "echo hello".into(),
            session: None,
            timeout: None,
            background: false,
        };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(result.output.trim(), "hello world");
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.timed_out);
        assert_eq!(result.session, "main");
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.set_should_timeout(true);
        }

        let args = BashArgs {
            command: "sleep 100".into(),
            session: None,
            timeout: Some(1),
            background: false,
        };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert!(result.timed_out);
        assert!(result.exit_code.is_none());
    }

    #[tokio::test]
    async fn test_bash_background_mode() {
        let (tool, _) = create_tool_with_mock();

        let args = BashArgs {
            command: "long-running-cmd".into(),
            session: None,
            timeout: None,
            background: true,
        };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert!(result.exit_code.is_none());
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn test_bash_specific_session() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.add_session("worker");
            backend.push_output("done");
            backend.push_exit_code(0);
        }

        let args = BashArgs {
            command: "echo done".into(),
            session: Some("worker".into()),
            timeout: None,
            background: false,
        };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(result.session, "worker");
    }

    #[tokio::test]
    async fn test_bash_nonexistent_session() {
        let (tool, _) = create_tool_with_mock();

        let args = BashArgs {
            command: "echo test".into(),
            session: Some("nonexistent".into()),
            timeout: None,
            background: false,
        };
        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is::<ShellError>());
    }

    #[tokio::test]
    async fn test_bash_nonzero_exit_code() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.push_output("error: something failed");
            backend.push_exit_code(1);
        }

        let args =
            BashArgs { command: "false".into(), session: None, timeout: None, background: false };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(result.exit_code, Some(1));
        assert!(result.output.contains("error"));
    }

    #[tokio::test]
    async fn test_bash_empty_command() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.set_default_output("");
            backend.set_default_exit_code(0);
        }

        let args = BashArgs { command: "".into(), session: None, timeout: None, background: false };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(result.session, "main");
    }

    #[tokio::test]
    async fn test_bash_empty_output() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.push_output("");
            backend.push_exit_code(0);
        }

        let args =
            BashArgs { command: "true".into(), session: None, timeout: None, background: false };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(result.output, "");
        assert_eq!(result.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_bash_special_characters() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.push_output("hello \"world\" $FOO 'bar'");
            backend.push_exit_code(0);
        }

        let args = BashArgs {
            command: "echo 'hello \"world\" $FOO '\\''bar'\\'''".into(),
            session: None,
            timeout: None,
            background: false,
        };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert!(result.output.contains("hello"));
        assert_eq!(result.exit_code, Some(0));
    }

    /// Test that commands and output containing non-ASCII characters are handled correctly.
    #[tokio::test]
    async fn test_bash_unicode_command() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.push_output("你好世界 🌍");
            backend.push_exit_code(0);
        }

        let args = BashArgs {
            command: "echo '你好世界 🌍'".into(),
            session: None,
            timeout: None,
            background: false,
        };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert!(result.output.contains("你好世界"));
        assert_eq!(result.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_bash_custom_timeout() {
        let (tool, mock) = create_tool_with_mock();
        {
            let mut backend = mock.lock().await;
            backend.push_output("done");
            backend.push_exit_code(0);
        }

        let args = BashArgs {
            command: "echo done".into(),
            session: None,
            timeout: Some(300),
            background: false,
        };
        let result: BashOutput = serde_json::from_str(
            &tool
                .call(&serde_json::to_string(&args).unwrap())
                .await
                .unwrap(),
        )
        .unwrap();

        assert_eq!(result.output, "done");
        assert_eq!(result.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_bash_name_and_schema() {
        let (tool, _) = create_tool_with_mock();
        assert_eq!(tool.name(), "bash");
        assert!(tool.description().contains("shell command"));
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["command"].is_object());
    }
}
