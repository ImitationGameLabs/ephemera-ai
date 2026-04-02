//! Tmux-based shell backend implementation
//!
//! This module provides a shell backend that uses tmux for session management
//! and command execution. Tmux handles PTY management, session persistence,
//! and terminal emulation.

use async_trait::async_trait;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

use tmux_interface::{KillSession, NewSession, Tmux};

use super::backend::{SessionInfo, ShellBackend, ShellOutput};
use super::error::ShellError;

/// Tmux-based shell backend
///
/// Uses tmux for persistent shell sessions with full terminal emulation.
/// Sessions persist across backend restarts and can be attached to manually
/// for debugging.
pub struct TmuxBackend {
    current_session: String,
    prompt_pattern: String,
}

impl TmuxBackend {
    /// Create a new tmux backend with a default session
    ///
    /// Creates a new tmux session with the given name if it doesn't exist.
    pub async fn new(default_name: &str) -> Result<Self, ShellError> {
        let backend = Self {
            current_session: default_name.to_string(),
            prompt_pattern: r"[$#]\s*$".to_string(), // Default bash/zsh prompt
        };

        // Create the default session if it doesn't exist
        if !backend.session_exists(default_name).await? {
            backend.create_session_internal(default_name, None).await?;
        }

        Ok(backend)
    }

    /// Check if a tmux session exists
    async fn session_exists(&self, name: &str) -> Result<bool, ShellError> {
        let sessions = self.list_sessions().await?;
        Ok(sessions.iter().any(|s| s.name == name))
    }

    /// Internal method to create a session
    async fn create_session_internal(
        &self,
        name: &str,
        cwd: Option<&Path>,
    ) -> Result<(), ShellError> {
        let cwd_str = cwd.map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/tmp".to_string())
        });

        let new_session = NewSession::new()
            .session_name(name)
            .shell_command("bash --login")
            .start_directory(&cwd_str);

        Tmux::new()
            .add_command(new_session)
            .output()
            .map_err(|e| ShellError::session_create_failed(name, e.to_string()))?;

        // Wait a bit for the session to initialize
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Wait for command completion using prompt detection + stability
    async fn wait_for_completion(
        &self,
        session: &str,
        timeout: Duration,
    ) -> Result<String, ShellError> {
        let start = std::time::Instant::now();
        let mut last_output = String::new();
        let mut stable_count = 0;
        let prompt_re = regex::Regex::new(&self.prompt_pattern)
            .unwrap_or_else(|_| regex::Regex::new(r"[$#]\s*$").unwrap());

        loop {
            let output = self.capture_output_internal(session, 100).await?;

            // Check for prompt and output stability
            let has_prompt = prompt_re.is_match(&output);
            if has_prompt && output == last_output {
                stable_count += 1;
                if stable_count >= 3 {
                    // Output stable for 3 consecutive checks
                    return Ok(output);
                }
            } else {
                stable_count = 0;
            }

            if start.elapsed() > timeout {
                return Err(ShellError::timeout(timeout.as_secs()));
            }

            last_output = output;
            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Capture output from a session (internal method)
    async fn capture_output_internal(
        &self,
        session: &str,
        lines: usize,
    ) -> Result<String, ShellError> {
        let output = Command::new("tmux")
            .args(["capture-pane", "-t", session, "-p", "-S", &format!("-{}", lines)])
            .output()
            .map_err(|e| ShellError::backend(format!("capture-pane failed: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[async_trait]
impl ShellBackend for TmuxBackend {
    async fn execute(
        &mut self,
        command: &str,
        timeout: Duration,
        background: bool,
    ) -> Result<ShellOutput, ShellError> {
        let session = self.current_session.clone();

        if !self.session_exists(&session).await? {
            return Err(ShellError::session_not_found(&session));
        }

        // Send the command using tmux command directly
        let output = Command::new("tmux")
            .args(["send-keys", "-t", &session, command])
            .output()
            .map_err(|e| ShellError::execution_failed(format!("send-keys failed: {}", e)))?;

        if !output.status.success() {
            return Err(ShellError::execution_failed("send-keys returned non-zero"));
        }

        // Press Enter
        let output = Command::new("tmux")
            .args(["send-keys", "-t", &session, "Enter"])
            .output()
            .map_err(|e| ShellError::execution_failed(format!("send-keys failed: {}", e)))?;

        if !output.status.success() {
            return Err(ShellError::execution_failed("send-keys Enter returned non-zero"));
        }

        if background {
            return Ok(ShellOutput { output: String::new(), exit_code: None, timed_out: false });
        }

        // Wait for command completion
        let output = match self.wait_for_completion(&session, timeout).await {
            Ok(o) => o,
            Err(ShellError::Timeout { timeout: _ }) => {
                return Ok(ShellOutput { output: String::new(), exit_code: None, timed_out: true });
            }
            Err(e) => return Err(e),
        };

        // Get exit code from last command
        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &session, "echo __EXIT_CODE__$?__EXIT_CODE__"])
            .output();

        let _ = Command::new("tmux").args(["send-keys", "-t", &session, "Enter"]).output();

        sleep(Duration::from_millis(200)).await;

        let exit_output = self.capture_output_internal(&session, 50).await?;

        // Parse exit code
        let exit_re =
            regex::Regex::new(r"__EXIT_CODE__(\d+)__EXIT_CODE__").expect("Invalid exit code regex");
        let exit_code = exit_re.captures(&exit_output).and_then(|caps| caps[1].parse::<i32>().ok());

        Ok(ShellOutput { output, exit_code, timed_out: false })
    }

    async fn send_input(&mut self, input: &str, press_enter: bool) -> Result<(), ShellError> {
        let session = self.current_session.clone();

        if !self.session_exists(&session).await? {
            return Err(ShellError::session_not_found(&session));
        }

        let _ = Command::new("tmux")
            .args(["send-keys", "-t", &session, input])
            .output()
            .map_err(|e| ShellError::backend(format!("send-keys failed: {}", e)))?;

        if press_enter {
            let _ = Command::new("tmux")
                .args(["send-keys", "-t", &session, "Enter"])
                .output()
                .map_err(|e| ShellError::backend(format!("send-keys failed: {}", e)))?;
        }

        sleep(Duration::from_millis(50)).await;
        Ok(())
    }

    async fn capture_output(&mut self, lines: usize) -> Result<String, ShellError> {
        let session = self.current_session.clone();
        self.capture_output_internal(&session, lines).await
    }

    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, ShellError> {
        let output = Command::new("tmux")
            .args([
                "list-sessions",
                "-F",
                "#{session_name}:#{session_path}:#{?session_attached,1,0}:#{window_count}",
            ])
            .output()
            .map_err(|e| ShellError::backend(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let sessions: Vec<SessionInfo> = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 4 {
                    Some(SessionInfo {
                        name: parts[0].to_string(),
                        cwd: parts[1].to_string(),
                        is_current: parts[0] == self.current_session,
                        window_count: parts[3].parse().unwrap_or(1),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(sessions)
    }

    async fn create_session(&mut self, name: &str, cwd: Option<&Path>) -> Result<(), ShellError> {
        if self.session_exists(name).await? {
            return Err(ShellError::session_exists(name));
        }

        self.create_session_internal(name, cwd).await
    }

    async fn switch_session(&mut self, name: &str) -> Result<(), ShellError> {
        if !self.session_exists(name).await? {
            return Err(ShellError::session_not_found(name));
        }

        self.current_session = name.to_string();
        Ok(())
    }

    async fn kill_session(&mut self, name: &str) -> Result<(), ShellError> {
        if !self.session_exists(name).await? {
            return Err(ShellError::session_not_found(name));
        }

        let kill_session = KillSession::new().target_session(name);

        Tmux::new()
            .add_command(kill_session)
            .output()
            .map_err(|e| ShellError::backend(format!("kill-session failed: {}", e)))?;

        // If we killed the current session, switch to another
        if self.current_session == name {
            let sessions = self.list_sessions().await?;
            self.current_session =
                sessions.first().map(|s| s.name.clone()).unwrap_or_else(|| "main".to_string());
        }

        Ok(())
    }

    async fn restart_session(&mut self, name: &str, _clean_env: bool) -> Result<(), ShellError> {
        if !self.session_exists(name).await? {
            return Err(ShellError::session_not_found(name));
        }

        // Kill the session
        self.kill_session(name).await?;

        // Recreate it
        self.create_session_internal(name, None).await?;

        Ok(())
    }

    fn current_session(&self) -> &str {
        &self.current_session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmux_backend_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TmuxBackend>();
    }
}
