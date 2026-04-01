//! Shell tool error types
//!
//! These errors are designed to be backend-agnostic - they don't expose
//! any implementation details about tmux, PTY, or other backends.
//! This allows swapping backends without changing the error types.

use thiserror::Error;

/// Errors that can occur during shell tool operations
#[derive(Debug, Error)]
pub enum ShellError {
    #[error("Command timed out after {timeout}s")]
    Timeout { timeout: u64 },

    #[error("Session '{name}' not found")]
    SessionNotFound { name: String },

    #[error("Session '{name}' already exists")]
    SessionExists { name: String },

    #[error("No sessions available")]
    NoSessions,

    #[error("Command execution failed: {reason}")]
    ExecutionFailed { reason: String },

    #[error("Backend error: {reason}")]
    BackendError { reason: String },

    #[error("Failed to create session '{name}': {reason}")]
    SessionCreateFailed { name: String, reason: String },

    #[error("IO error: {0}")]
    Io(String),
}

impl ShellError {
    /// Create a timeout error
    pub fn timeout(seconds: u64) -> Self {
        ShellError::Timeout { timeout: seconds }
    }

    /// Create a session not found error
    pub fn session_not_found(name: impl Into<String>) -> Self {
        ShellError::SessionNotFound { name: name.into() }
    }

    /// Create a session exists error
    pub fn session_exists(name: impl Into<String>) -> Self {
        ShellError::SessionExists { name: name.into() }
    }

    /// Create an execution failed error
    pub fn execution_failed(reason: impl Into<String>) -> Self {
        ShellError::ExecutionFailed { reason: reason.into() }
    }

    /// Create a backend error
    pub fn backend(reason: impl Into<String>) -> Self {
        ShellError::BackendError { reason: reason.into() }
    }

    /// Create a session creation failed error
    pub fn session_create_failed(name: impl Into<String>, reason: impl Into<String>) -> Self {
        ShellError::SessionCreateFailed { name: name.into(), reason: reason.into() }
    }

    /// Create an IO error (hides implementation details)
    pub fn io(reason: impl Into<String>) -> Self {
        ShellError::Io(reason.into())
    }
}

impl From<std::io::Error> for ShellError {
    fn from(err: std::io::Error) -> Self {
        // Don't expose std::io::Error details, just the message
        ShellError::Io(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ShellError::timeout(30);
        assert_eq!(err.to_string(), "Command timed out after 30s");

        let err = ShellError::session_not_found("worker");
        assert_eq!(err.to_string(), "Session 'worker' not found");

        let err = ShellError::session_exists("main");
        assert_eq!(err.to_string(), "Session 'main' already exists");

        let err = ShellError::NoSessions;
        assert_eq!(err.to_string(), "No sessions available");

        let err = ShellError::execution_failed("command not found");
        assert_eq!(err.to_string(), "Command execution failed: command not found");

        let err = ShellError::backend("tmux not installed");
        assert_eq!(err.to_string(), "Backend error: tmux not installed");

        let err = ShellError::session_create_failed("test", "permission denied");
        assert_eq!(err.to_string(), "Failed to create session 'test': permission denied");

        let err = ShellError::Io("file not found".into());
        assert_eq!(err.to_string(), "IO error: file not found");
    }

    #[test]
    fn test_error_constructors() {
        let err = ShellError::timeout(60);
        assert!(matches!(err, ShellError::Timeout { timeout: 60 }));

        let err = ShellError::session_not_found("my_session");
        assert!(matches!(
            err,
            ShellError::SessionNotFound { name } if name == "my_session"
        ));

        let err = ShellError::session_exists("existing");
        assert!(matches!(
            err,
            ShellError::SessionExists { name } if name == "existing"
        ));

        let err = ShellError::execution_failed("some error");
        assert!(matches!(
            err,
            ShellError::ExecutionFailed { reason } if reason == "some error"
        ));

        let err = ShellError::backend("backend issue");
        assert!(matches!(
            err,
            ShellError::BackendError { reason } if reason == "backend issue"
        ));

        let err = ShellError::session_create_failed("sess", "reason");
        assert!(matches!(
            err,
            ShellError::SessionCreateFailed { name, reason } if name == "sess" && reason == "reason"
        ));

        let err = ShellError::io("io issue");
        assert!(matches!(err, ShellError::Io(msg) if msg == "io issue"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let shell_err: ShellError = io_err.into();
        assert!(matches!(shell_err, ShellError::Io(_)));
    }
}
