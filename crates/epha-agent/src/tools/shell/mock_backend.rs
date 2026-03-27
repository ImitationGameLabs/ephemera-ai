//! Mock shell backend for testing
//!
//! This mock implementation allows unit tests to verify tool behavior
//! without requiring tmux or any external dependencies.

use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::backend::{SessionInfo, ShellBackend, ShellOutput};
use super::error::ShellError;

/// A mock session for testing
#[derive(Debug, Clone)]
pub struct MockSession {
    pub name: String,
    pub cwd: String,
    pub env: HashMap<String, String>,
    pub history: Vec<String>,
    pub pending_output: VecDeque<String>,
}

impl MockSession {
    pub fn new(name: &str, cwd: &str) -> Self {
        Self {
            name: name.to_string(),
            cwd: cwd.to_string(),
            env: HashMap::new(),
            history: Vec::new(),
            pending_output: VecDeque::new(),
        }
    }
}

/// Mock shell backend for testing
///
/// This backend simulates shell behavior without actually executing commands.
/// It can be configured with predefined outputs, exit codes, and behaviors.
#[derive(Debug)]
pub struct MockShellBackend {
    sessions: HashMap<String, MockSession>,
    current: String,
    /// Queue of outputs to return for subsequent execute calls
    next_outputs: VecDeque<String>,
    /// Queue of exit codes to return
    next_exit_codes: VecDeque<i32>,
    /// If true, next execute will timeout
    should_timeout: bool,
    /// All inputs received via send_input
    inputs_received: Arc<Mutex<Vec<String>>>,
    /// Default output when queues are empty
    default_output: String,
    /// Default exit code when queue is empty
    default_exit_code: i32,
}

impl Default for MockShellBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MockShellBackend {
    /// Create a new mock backend with a default "main" session
    pub fn new() -> Self {
        let mut sessions = HashMap::new();
        sessions.insert(
            "main".to_string(),
            MockSession::new(
                "main",
                std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "/tmp".to_string())
                    .as_str(),
            ),
        );

        Self {
            sessions,
            current: "main".to_string(),
            next_outputs: VecDeque::new(),
            next_exit_codes: VecDeque::new(),
            should_timeout: false,
            inputs_received: Arc::new(Mutex::new(Vec::new())),
            default_output: String::new(),
            default_exit_code: 0,
        }
    }

    /// Add a predefined output for the next execute call
    pub fn push_output(&mut self, output: impl Into<String>) -> &mut Self {
        self.next_outputs.push_back(output.into());
        self
    }

    /// Add a predefined exit code for the next execute call
    pub fn push_exit_code(&mut self, code: i32) -> &mut Self {
        self.next_exit_codes.push_back(code);
        self
    }

    /// Set whether the next execute should timeout
    pub fn set_should_timeout(&mut self, should: bool) -> &mut Self {
        self.should_timeout = should;
        self
    }

    /// Set the default output when queues are empty
    pub fn set_default_output(&mut self, output: impl Into<String>) -> &mut Self {
        self.default_output = output.into();
        self
    }

    /// Set the default exit code when queue is empty
    pub fn set_default_exit_code(&mut self, code: i32) -> &mut Self {
        self.default_exit_code = code;
        self
    }

    /// Add a session to the mock backend
    pub fn add_session(&mut self, name: &str) -> &mut Self {
        self.sessions.insert(name.to_string(), MockSession::new(name, "/tmp"));
        self
    }

    /// Add a session with a specific cwd
    pub fn add_session_with_cwd(&mut self, name: &str, cwd: &str) -> &mut Self {
        self.sessions.insert(name.to_string(), MockSession::new(name, cwd));
        self
    }

    /// Set the current session
    pub fn set_current(&mut self, name: &str) -> &mut Self {
        self.current = name.to_string();
        self
    }

    /// Set an environment variable in a session
    pub fn set_env(&mut self, session: &str, key: &str, value: &str) -> &mut Self {
        if let Some(sess) = self.sessions.get_mut(session) {
            sess.env.insert(key.to_string(), value.to_string());
        }
        self
    }

    /// Check if a session has an environment variable
    pub fn has_env(&self, session: &str, key: &str) -> bool {
        self.sessions.get(session).map(|s| s.env.contains_key(key)).unwrap_or(false)
    }

    /// Get all inputs received via send_input
    pub fn get_inputs_received(&self) -> Vec<String> {
        self.inputs_received.lock().unwrap().clone()
    }

    /// Clear all received inputs
    pub fn clear_inputs(&self) {
        self.inputs_received.lock().unwrap().clear();
    }

    /// Check if a session exists
    pub fn has_session(&self, name: &str) -> bool {
        self.sessions.contains_key(name)
    }

    /// Get the number of sessions
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Set pending output for a session (for capture_output)
    pub fn set_session_output(&mut self, session: &str, lines: Vec<&str>) -> &mut Self {
        if let Some(sess) = self.sessions.get_mut(session) {
            sess.pending_output = lines.into_iter().map(String::from).collect();
        }
        self
    }
}

#[async_trait]
impl ShellBackend for MockShellBackend {
    async fn execute(
        &mut self,
        command: &str,
        _timeout: Duration,
        background: bool,
    ) -> Result<ShellOutput, ShellError> {
        // Check if current session exists
        if !self.sessions.contains_key(&self.current) {
            return Err(ShellError::session_not_found(&self.current));
        }

        // Record command in history
        if let Some(session) = self.sessions.get_mut(&self.current) {
            session.history.push(command.to_string());
        }

        // Check for timeout
        if self.should_timeout {
            self.should_timeout = false;
            return Ok(ShellOutput { output: String::new(), exit_code: None, timed_out: true });
        }

        // Background mode returns immediately without exit code
        if background {
            return Ok(ShellOutput { output: String::new(), exit_code: None, timed_out: false });
        }

        // Get output from queue or use default
        let output = self.next_outputs.pop_front().unwrap_or_else(|| self.default_output.clone());

        // Get exit code from queue or use default
        let exit_code = self.next_exit_codes.pop_front().unwrap_or(self.default_exit_code);

        Ok(ShellOutput { output, exit_code: Some(exit_code), timed_out: false })
    }

    async fn send_input(&mut self, input: &str, press_enter: bool) -> Result<(), ShellError> {
        if !self.sessions.contains_key(&self.current) {
            return Err(ShellError::session_not_found(&self.current));
        }

        let mut received = input.to_string();
        if press_enter {
            received.push('\n');
        }

        self.inputs_received.lock().unwrap().push(received);
        Ok(())
    }

    async fn capture_output(&mut self, lines: usize) -> Result<String, ShellError> {
        let session = self
            .sessions
            .get(&self.current)
            .ok_or_else(|| ShellError::session_not_found(&self.current))?;

        let output: String = session
            .pending_output
            .iter()
            .rev()
            .take(lines)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");

        Ok(output)
    }

    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, ShellError> {
        let sessions: Vec<SessionInfo> = self
            .sessions
            .values()
            .map(|s| SessionInfo {
                name: s.name.clone(),
                cwd: s.cwd.clone(),
                is_current: s.name == self.current,
                window_count: 1,
            })
            .collect();

        Ok(sessions)
    }

    async fn create_session(&mut self, name: &str, cwd: Option<&Path>) -> Result<(), ShellError> {
        if self.sessions.contains_key(name) {
            return Err(ShellError::session_exists(name));
        }

        let cwd_str =
            cwd.map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| "/tmp".to_string());

        self.sessions.insert(name.to_string(), MockSession::new(name, &cwd_str));
        Ok(())
    }

    async fn switch_session(&mut self, name: &str) -> Result<(), ShellError> {
        if !self.sessions.contains_key(name) {
            return Err(ShellError::session_not_found(name));
        }

        self.current = name.to_string();
        Ok(())
    }

    async fn kill_session(&mut self, name: &str) -> Result<(), ShellError> {
        if !self.sessions.contains_key(name) {
            return Err(ShellError::session_not_found(name));
        }

        self.sessions.remove(name);

        // If we killed the current session, switch to another
        if self.current == name {
            self.current =
                self.sessions.keys().next().cloned().unwrap_or_else(|| "main".to_string());
        }

        Ok(())
    }

    async fn restart_session(&mut self, name: &str, clean_env: bool) -> Result<(), ShellError> {
        if !self.sessions.contains_key(name) {
            return Err(ShellError::session_not_found(name));
        }

        if let Some(session) = self.sessions.get_mut(name) {
            session.history.clear();
            if clean_env {
                session.env.clear();
            }
        }

        Ok(())
    }

    fn current_session(&self) -> &str {
        &self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_mock_backend_new() {
        let backend = MockShellBackend::new();
        assert!(backend.has_session("main"));
        assert_eq!(backend.current_session(), "main");
    }

    #[tokio::test]
    async fn test_backend_concurrent_access() {
        // Test Arc<Mutex<Backend>> concurrent access
        let mock = Arc::new(Mutex::new(MockShellBackend::new()));
        let mut handles = vec![];

        for i in 0..10 {
            let m = mock.clone();
            handles.push(tokio::spawn(async move {
                let mut backend = m.lock().await;
                backend.execute(&format!("cmd {}", i), Duration::from_secs(1), false).await
            }));
        }

        for handle in handles {
            let _ = handle.await;
        }
    }

    #[tokio::test]
    async fn test_mock_execute_basic() {
        let mut backend = MockShellBackend::new();
        backend.push_output("hello world");
        backend.push_exit_code(0);

        let result = backend.execute("echo hello", Duration::from_secs(10), false).await.unwrap();

        assert_eq!(result.output, "hello world");
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn test_mock_execute_timeout() {
        let mut backend = MockShellBackend::new();
        backend.set_should_timeout(true);

        let result = backend.execute("sleep 100", Duration::from_secs(1), false).await.unwrap();

        assert!(result.timed_out);
        assert!(result.exit_code.is_none());
    }

    #[tokio::test]
    async fn test_mock_execute_background() {
        let mut backend = MockShellBackend::new();

        let result = backend.execute("long-cmd", Duration::from_secs(10), true).await.unwrap();

        assert!(result.exit_code.is_none());
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn test_mock_session_management() {
        let mut backend = MockShellBackend::new();

        // Add session
        backend.add_session("worker");
        assert!(backend.has_session("worker"));
        assert_eq!(backend.session_count(), 2);

        // Switch session
        backend.switch_session("worker").await.unwrap();
        assert_eq!(backend.current_session(), "worker");

        // Kill session
        backend.kill_session("worker").await.unwrap();
        assert!(!backend.has_session("worker"));
        assert_eq!(backend.session_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_send_input() {
        let mut backend = MockShellBackend::new();

        backend.send_input("y", true).await.unwrap();
        backend.send_input("password", false).await.unwrap();

        let inputs = backend.get_inputs_received();
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0], "y\n");
        assert_eq!(inputs[1], "password");
    }

    #[tokio::test]
    async fn test_mock_list_sessions() {
        let mut backend = MockShellBackend::new();
        backend.add_session("build");
        backend.add_session("test");

        let sessions = backend.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 3);

        let main_session = sessions.iter().find(|s| s.name == "main").unwrap();
        assert!(main_session.is_current);
    }

    #[tokio::test]
    async fn test_mock_session_not_found() {
        let mut backend = MockShellBackend::new();
        backend.set_current("nonexistent");

        let result = backend.execute("echo test", Duration::from_secs(10), false).await;

        assert!(matches!(result, Err(ShellError::SessionNotFound { .. })));
    }

    #[tokio::test]
    async fn test_mock_create_duplicate_session() {
        let mut backend = MockShellBackend::new();

        let result = backend.create_session("main", None).await;
        assert!(matches!(result, Err(ShellError::SessionExists { .. })));
    }

    #[tokio::test]
    async fn test_mock_restart_session() {
        let mut backend = MockShellBackend::new();
        backend.set_env("main", "MY_VAR", "test");
        assert!(backend.has_env("main", "MY_VAR"));

        backend.restart_session("main", true).await.unwrap();
        assert!(!backend.has_env("main", "MY_VAR"));
    }

    #[tokio::test]
    async fn test_mock_capture_output() {
        let mut backend = MockShellBackend::new();
        backend.set_session_output("main", vec!["line1", "line2", "line3"]);

        let output = backend.capture_output(2).await.unwrap();
        assert_eq!(output, "line2\nline3");
    }

    #[tokio::test]
    async fn test_mock_execute_queue_consumption() {
        // Test multiple consecutive execute calls consume queue properly
        let mut backend = MockShellBackend::new();
        backend.push_output("first");
        backend.push_exit_code(0);
        backend.push_output("second");
        backend.push_exit_code(1);
        backend.push_output("third");
        backend.push_exit_code(2);

        let result1 = backend.execute("cmd1", Duration::from_secs(1), false).await.unwrap();
        assert_eq!(result1.output, "first");
        assert_eq!(result1.exit_code, Some(0));

        let result2 = backend.execute("cmd2", Duration::from_secs(1), false).await.unwrap();
        assert_eq!(result2.output, "second");
        assert_eq!(result2.exit_code, Some(1));

        let result3 = backend.execute("cmd3", Duration::from_secs(1), false).await.unwrap();
        assert_eq!(result3.output, "third");
        assert_eq!(result3.exit_code, Some(2));
    }

    #[tokio::test]
    async fn test_mock_default_output_and_exit_code() {
        // Test default output/exit code when queues are empty
        let mut backend = MockShellBackend::new();
        backend.set_default_output("default output");
        backend.set_default_exit_code(42);

        // Queue is empty, should use defaults
        let result1 = backend.execute("cmd1", Duration::from_secs(1), false).await.unwrap();
        assert_eq!(result1.output, "default output");
        assert_eq!(result1.exit_code, Some(42));

        // Still using defaults
        let result2 = backend.execute("cmd2", Duration::from_secs(1), false).await.unwrap();
        assert_eq!(result2.output, "default output");
        assert_eq!(result2.exit_code, Some(42));
    }

    #[tokio::test]
    async fn test_mock_kill_last_session() {
        // Test killing the only session
        let mut backend = MockShellBackend::new();
        assert!(backend.has_session("main"));

        backend.kill_session("main").await.unwrap();

        // After killing the only session, current_session returns "main" as fallback
        // (since there are no other sessions to switch to)
        assert!(!backend.has_session("main"));
        // The backend falls back to "main" when no sessions exist
        assert_eq!(backend.current_session(), "main");
    }

    #[tokio::test]
    async fn test_mock_capture_output_empty_session() {
        let mut backend = MockShellBackend::new();
        // No output set for session
        let output = backend.capture_output(100).await.unwrap();
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn test_mock_capture_output_zero_lines() {
        let mut backend = MockShellBackend::new();
        backend.set_session_output("main", vec!["line1", "line2", "line3"]);

        let output = backend.capture_output(0).await.unwrap();
        assert!(output.is_empty());
    }

    #[test]
    fn test_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockShellBackend>();
    }
}
