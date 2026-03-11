//! Configuration for kairos-herald.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Kairos service URL
    pub kairos_url: String,
    /// Agora service URL
    pub agora_url: String,
    /// Poll interval for triggered schedules (milliseconds)
    pub poll_interval_ms: u64,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_sec: u64,
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read config '{}': {}", path.display(), e));
        let config: Self = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse config '{}': {}", path.display(), e));

        // Validate required fields
        assert!(
            !config.kairos_url.trim().is_empty(),
            "kairos_url cannot be empty"
        );
        assert!(
            !config.agora_url.trim().is_empty(),
            "agora_url cannot be empty"
        );
        assert!(
            config.poll_interval_ms > 0,
            "poll_interval_ms must be greater than 0"
        );
        assert!(
            config.heartbeat_interval_sec > 0,
            "heartbeat_interval_sec must be greater than 0"
        );

        config
    }
}
