//! Configuration schema for Agora.

use serde::Deserialize;
use std::path::Path;

/// Agora server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server listen port.
    pub port: u16,
    /// Path to SQLite database file.
    pub database_path: String,
    /// Interval between heartbeat timeout checks in milliseconds
    pub heartbeat_check_interval_ms: u64,
    /// Milliseconds without heartbeat before marking herald as Disconnected
    pub timeout_ms: i64,
    /// Retry configuration for event delivery.
    #[serde(default)]
    pub retry: RetryConfig,
}

/// Retry configuration for event delivery.
#[derive(Debug, Clone, Deserialize)]
pub struct RetryConfig {
    /// Initial retry interval (ms), default: 5000
    #[serde(default = "default_base_interval")]
    pub base_interval_ms: u64,
    /// Multiplier for each retry, default: 2
    #[serde(default = "default_multiplier")]
    pub multiplier: u32,
    /// Max retry interval (ms), default: 300000 (5min)
    #[serde(default = "default_max_interval")]
    pub max_interval_ms: u64,
}

fn default_base_interval() -> u64 {
    5000
}
fn default_multiplier() -> u32 {
    2
}
fn default_max_interval() -> u64 {
    300000
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            base_interval_ms: default_base_interval(),
            multiplier: default_multiplier(),
            max_interval_ms: default_max_interval(),
        }
    }
}

impl Config {
    /// Loads configuration from a JSON file.
    ///
    /// # Panics
    ///
    /// Panics if the file cannot be read or parsed, or if required fields are invalid.
    pub fn load(path: &Path) -> Self {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read config '{}': {}", path.display(), e));
        let config: Self = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse config '{}': {}", path.display(), e));

        // Validate required fields
        assert!(config.port != 0, "port cannot be 0");
        assert!(
            !config.database_path.is_empty(),
            "database_path cannot be empty"
        );
        assert!(
            config.heartbeat_check_interval_ms > 0,
            "heartbeat_check_interval_ms must be greater than 0"
        );
        assert!(config.timeout_ms > 0, "timeout_ms must be greater than 0");

        config
    }

    /// Returns the bind address for the server.
    pub fn bind_address(&self) -> String {
        format!("[::]:{}", self.port)
    }
}
