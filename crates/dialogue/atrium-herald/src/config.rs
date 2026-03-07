//! Configuration for atrium-herald.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Atrium service URL
    pub atrium_url: String,
    /// Agora service URL
    pub agora_url: String,
    /// Atrium login username
    pub username: String,
    /// Atrium login password
    pub password: String,
    /// Message poll interval in milliseconds
    pub poll_interval_ms: u64,
    /// Agora heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Atrium heartbeat interval in milliseconds (user online status)
    pub atrium_heartbeat_interval_ms: u64,
    /// Bio for user registration (optional)
    pub bio: Option<String>,
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read config '{}': {}", path.display(), e));
        let config: Self = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse config '{}': {}", path.display(), e));

        // Validate required fields
        assert!(
            !config.atrium_url.trim().is_empty(),
            "atrium_url cannot be empty"
        );
        assert!(
            !config.agora_url.trim().is_empty(),
            "agora_url cannot be empty"
        );
        assert!(
            !config.username.trim().is_empty(),
            "username cannot be empty"
        );
        assert!(
            !config.password.trim().is_empty(),
            "password cannot be empty"
        );
        assert!(
            config.poll_interval_ms > 0,
            "poll_interval_ms must be greater than 0"
        );
        assert!(
            config.heartbeat_interval_ms > 0,
            "heartbeat_interval_ms must be greater than 0"
        );
        assert!(
            config.atrium_heartbeat_interval_ms > 0,
            "atrium_heartbeat_interval_ms must be greater than 0"
        );

        config
    }
}
