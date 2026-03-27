use serde::Deserialize;
use std::path::Path;

/// MySQL database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MySqlConfig {
    /// MySQL server URL (e.g., "mysql://user:password@localhost:3306/loom")
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mysql: MySqlConfig,
    pub port: u16,
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read config '{}': {}", path.display(), e));
        let config: Self = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse config '{}': {}", path.display(), e));

        // Validate required fields
        assert!(!config.mysql.url.trim().is_empty(), "mysql.url cannot be empty");
        assert!(config.port != 0, "port cannot be 0");

        config
    }

    pub fn bind_address(&self) -> String {
        format!("[::]:{}", self.port)
    }
}
