//! Shell tools for agents
//!
//! This module provides a set of shell operation tools based on tmux,
//! supporting persistent sessions, multi-session management, and
//! interactive operations.
//!
//! # Architecture
//!
//! The shell tools use a backend abstraction pattern:
//!
//! ```text
//! AI → Tools → ShellBackend trait → TmuxBackend → tmux → shell
//!                  ↑                      ↑
//!          abstract interface      one implementation (swappable)
//! ```
//!
//! This allows swapping backends (tmux, PTY, Docker, SSH) without
//! changing tool implementations.
//!
//! # Available Tools
//!
//! ## Execution Tools
//! - [`BashTool`] - Execute commands with timeout and background support
//! - [`SendInputTool`] - Send input to interactive commands
//! - [`CaptureOutputTool`] - Capture output from sessions
//!
//! ## Session Management Tools
//! - [`ListSessionsTool`] - List all sessions
//! - [`CreateSessionTool`] - Create new sessions
//! - [`SwitchSessionTool`] - Switch between sessions
//! - [`KillSessionTool`] - Terminate sessions
//! - [`RestartSessionTool`] - Restart sessions with clean state
//!
//! # Example
//!
//! ```rust,ignore
//! use crate::tools::shell::{BashTool, shell_tool_set};
//! use crate::tools::shell::tmux_backend::TmuxBackend;
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//!
//! // Create backend
//! let backend = Arc::new(Mutex::new(TmuxBackend::new("main").await?));
//!
//! // Use individual tools
//! let bash_tool = BashTool::new(backend.clone());
//!
//! // Or get all shell tools as a vector
//! let tools = shell_tool_set(backend).await;
//! ```

mod backend;
mod bash;
mod capture_output;
mod error;
mod mock_backend;
mod send_input;
mod session;
mod tmux_backend;

// Re-export public types
pub use backend::{SessionInfo, ShellBackend, ShellOutput};
pub use error::ShellError;
pub use mock_backend::MockShellBackend;
pub use tmux_backend::TmuxBackend;

// Execution tools
pub use bash::{BashArgs, BashOutput, BashTool};
pub use capture_output::{CaptureOutputArgs, CaptureOutputOutput, CaptureOutputTool};
pub use send_input::{SendInputArgs, SendInputOutput, SendInputTool};

// Session management tools
pub use session::{
    CreateSessionArgs, CreateSessionOutput, CreateSessionTool, KillSessionArgs, KillSessionOutput,
    KillSessionTool, ListSessionsArgs, ListSessionsOutput, ListSessionsTool, RestartSessionArgs,
    RestartSessionOutput, RestartSessionTool, SwitchSessionArgs, SwitchSessionOutput,
    SwitchSessionTool,
};

use crate::tools::AgentTool;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Create a set of all shell tools with the given backend
///
/// This function creates all shell tools configured with the same backend.
///
/// # Arguments
/// * `backend` - Shared backend instance wrapped in Arc<Mutex>
///
/// # Returns
/// A vector of boxed tools implementing AgentTool
pub fn shell_tool_set<B: ShellBackend + Send + Sync + 'static>(
    backend: Arc<Mutex<B>>,
) -> Vec<Box<dyn AgentTool>> {
    vec![
        Box::new(BashTool::new(backend.clone())),
        Box::new(SendInputTool::new(backend.clone())),
        Box::new(CaptureOutputTool::new(backend.clone())),
        Box::new(ListSessionsTool::new(backend.clone())),
        Box::new(CreateSessionTool::new(backend.clone())),
        Box::new(SwitchSessionTool::new(backend.clone())),
        Box::new(KillSessionTool::new(backend.clone())),
        Box::new(RestartSessionTool::new(backend)),
    ]
}

/// Create a set of shell tools with a mock backend (for testing)
///
/// This is a convenience function for tests that don't need real tmux.
#[cfg(test)]
pub fn mock_shell_tool_set() -> (Vec<Box<dyn AgentTool>>, Arc<Mutex<MockShellBackend>>) {
    let backend = Arc::new(Mutex::new(MockShellBackend::new()));
    let tools = shell_tool_set(backend.clone());
    (tools, backend)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_tool_set_count() {
        let (tools, _) = mock_shell_tool_set();
        assert_eq!(tools.len(), 8);
    }

    #[tokio::test]
    async fn test_tool_set_with_mock_backend() {
        let backend = Arc::new(Mutex::new(MockShellBackend::new()));
        let tools = shell_tool_set(backend.clone());

        // Verify all tools were created
        assert_eq!(tools.len(), 8);
    }
}
