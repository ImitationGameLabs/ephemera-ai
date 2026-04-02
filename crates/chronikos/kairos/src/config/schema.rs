//! Configuration schema for Kairos.

use serde::Deserialize;
use std::path::Path;

/// Kairos server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server listen port.
    pub port: u16,
    /// SQLite database path.
    pub database_path: String,
    /// Interval for checking scheduled events (milliseconds).
    pub tick_interval_ms: u64,
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
            config.tick_interval_ms > 0,
            "tick_interval_ms must be greater than 0"
        );

        config
    }

    /// Returns the bind address for the server.
    pub fn bind_address(&self) -> String {
        format!("[::]:{}", self.port)
    }
}
