use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub services: ServicesConfig,
    pub atrium_auth: AtriumAuthConfig,
    /// Tick interval in milliseconds when in Dormant state
    pub dormant_tick_interval_ms: u64,
    /// Agora event hub configuration
    pub agora: AgoraConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServicesConfig {
    pub loom_url: String,
    pub atrium_url: String,
    pub loom_vector_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AtriumAuthConfig {
    pub username: String,
    pub password: String,
}

/// Agora event hub configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AgoraConfig {
    /// Agora service URL (e.g., "http://localhost:3000")
    pub url: String,
}

impl Config {
    pub fn load(path: &Path) -> Self {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read config '{}': {}", path.display(), e));
        let config: Self = serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse config '{}': {}", path.display(), e));

        // Validate required fields
        assert!(
            !config.llm.base_url.trim().is_empty(),
            "llm.base_url cannot be empty"
        );
        assert!(
            !config.llm.model.trim().is_empty(),
            "llm.model cannot be empty"
        );
        assert!(
            !config.llm.api_key.trim().is_empty(),
            "llm.api_key cannot be empty"
        );
        assert!(
            !config.services.loom_url.trim().is_empty(),
            "services.loom_url cannot be empty"
        );
        assert!(
            !config.services.atrium_url.trim().is_empty(),
            "services.atrium_url cannot be empty"
        );
        assert!(
            !config.services.loom_vector_url.trim().is_empty(),
            "services.loom_vector_url cannot be empty"
        );
        assert!(
            !config.atrium_auth.username.trim().is_empty(),
            "atrium_auth.username cannot be empty"
        );
        assert!(
            !config.atrium_auth.password.trim().is_empty(),
            "atrium_auth.password cannot be empty"
        );
        assert!(
            config.dormant_tick_interval_ms > 0,
            "dormant_tick_interval_ms cannot be empty or 0"
        );
        assert!(
            !config.agora.url.trim().is_empty(),
            "agora.url cannot be empty"
        );

        config
    }
}
