//! Configuration schema for Agora.

use serde::Deserialize;
use std::path::Path;

/// Agora server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Server listen port.
    pub port: u16,
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

        config
    }

    /// Returns the bind address for the server.
    pub fn bind_address(&self) -> String {
        format!("[::]:{}", self.port)
    }
}
