//! ShellBackend trait - abstract interface for shell execution backends
//!
//! This trait defines the interface that all shell backends must implement.
//! The abstraction allows swapping backends (tmux, PTY, Docker, SSH) without
//! changing tool implementations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

use super::error::ShellError;

/// Information about a shell session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session name
    pub name: String,
    /// Working directory
    pub cwd: String,
    /// Whether this is the currently focused session
    pub is_current: bool,
    /// Number of windows in the session
    pub window_count: usize,
}

/// Output from a command execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellOutput {
    /// Command output (stdout + stderr combined)
    pub output: String,
    /// Exit code (None if command was killed or is still running)
    pub exit_code: Option<i32>,
    /// Whether the command timed out
    pub timed_out: bool,
}

/// Trait for shell execution backends
///
/// This trait abstracts the shell execution layer, allowing different
/// implementations (tmux, PTY, Docker, SSH) to be used interchangeably.
///
/// All implementations must be `Send + Sync` for thread-safe access.
#[async_trait]
pub trait ShellBackend: Send + Sync {
    /// Execute a command in the current session
    ///
    /// # Arguments
    /// * `command` - The command to execute
    /// * `timeout` - Maximum time to wait for command completion
    /// * `background` - If true, return immediately without waiting for completion
    ///
    /// # Returns
    /// * `Ok(ShellOutput)` - Command output and status
    /// * `Err(ShellError::Timeout)` - Command exceeded timeout
    /// * `Err(ShellError::ExecutionFailed)` - Command failed to start
    async fn execute(
        &mut self,
        command: &str,
        timeout: Duration,
        background: bool,
    ) -> Result<ShellOutput, ShellError>;

    /// Send input to the running command in the current session
    ///
    /// This is used for interactive commands that require user input,
    /// such as sudo password prompts or y/n confirmations.
    ///
    /// # Arguments
    /// * `input` - The input text to send
    /// * `press_enter` - Whether to press Enter after sending the input
    async fn send_input(&mut self, input: &str, press_enter: bool) -> Result<(), ShellError>;

    /// Capture output from the current session
    ///
    /// Returns the last N lines of output from the session's terminal.
    ///
    /// # Arguments
    /// * `lines` - Maximum number of lines to capture
    async fn capture_output(&mut self, lines: usize) -> Result<String, ShellError>;

    /// List all available sessions
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, ShellError>;

    /// Create a new session
    ///
    /// # Arguments
    /// * `name` - Unique session name
    /// * `cwd` - Optional working directory (defaults to current directory)
    async fn create_session(&mut self, name: &str, cwd: Option<&Path>) -> Result<(), ShellError>;

    /// Switch to a different session
    ///
    /// After switching, all subsequent commands will run in the specified session.
    async fn switch_session(&mut self, name: &str) -> Result<(), ShellError>;

    /// Kill a session
    ///
    /// Terminates the session and all processes running within it.
    /// If the killed session is the current session, switches to another available session.
    async fn kill_session(&mut self, name: &str) -> Result<(), ShellError>;

    /// Restart a session
    ///
    /// Kills and recreates the session with a clean environment.
    ///
    /// # Arguments
    /// * `name` - Session to restart
    /// * `clean_env` - If true, clear all environment variables
    async fn restart_session(&mut self, name: &str, clean_env: bool) -> Result<(), ShellError>;

    /// Get the name of the current (focused) session
    fn current_session(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_info() {
        let info = SessionInfo {
            name: "main".to_string(),
            cwd: "/home/user".to_string(),
            is_current: true,
            window_count: 2,
        };

        assert_eq!(info.name, "main");
        assert_eq!(info.cwd, "/home/user");
        assert!(info.is_current);
        assert_eq!(info.window_count, 2);
    }

    #[test]
    fn test_shell_output() {
        let output =
            ShellOutput { output: "hello world".to_string(), exit_code: Some(0), timed_out: false };

        assert_eq!(output.output, "hello world");
        assert_eq!(output.exit_code, Some(0));
        assert!(!output.timed_out);
    }

    #[test]
    fn test_shell_output_timeout() {
        let output =
            ShellOutput { output: "partial output".to_string(), exit_code: None, timed_out: true };

        assert!(output.timed_out);
        assert!(output.exit_code.is_none());
    }
}
