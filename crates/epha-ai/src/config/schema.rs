use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub llm: LlmConfig,
    pub services: ServicesConfig,
    /// Tick interval in milliseconds when in Dormant state
    pub dormant_tick_interval_ms: u64,
    /// Agora event hub configuration
    pub agora: AgoraConfig,
    /// Context management configuration
    pub context: ContextConfig,
}

/// Context management configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ContextConfig {
    /// Maximum token budget for pinned memories
    pub max_pinned_tokens: usize,
    /// Total token budget floor - eviction stops at this level (includes all components)
    pub total_token_floor: usize,
    /// Total token budget ceiling - eviction triggers at this level (includes all components)
    pub total_token_ceiling: usize,
    /// Tokens reserved for LLM response output
    pub response_reserve_tokens: usize,
    /// Minimum number of recent activities to preserve during eviction
    pub min_activities: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    /// Maximum number of tool call iterations per cognitive cycle
    pub max_turns: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServicesConfig {
    pub loom_url: String,
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
            config.dormant_tick_interval_ms > 0,
            "dormant_tick_interval_ms cannot be empty or 0"
        );
        assert!(
            !config.agora.url.trim().is_empty(),
            "agora.url cannot be empty"
        );
        assert!(
            config.context.max_pinned_tokens > 0,
            "context.max_pinned_tokens must be greater than 0"
        );
        assert!(
            config.llm.max_turns > 0,
            "llm.max_turns must be greater than 0"
        );

        // Validate token limits
        let ctx = &config.context;
        assert!(
            ctx.total_token_floor < ctx.total_token_ceiling,
            "context.total_token_floor ({}) must be less than total_token_ceiling ({})",
            ctx.total_token_floor,
            ctx.total_token_ceiling
        );
        assert!(
            ctx.min_activities > 0,
            "context.min_activities must be greater than 0"
        );
        assert!(
            ctx.response_reserve_tokens > 0,
            "context.response_reserve_tokens must be greater than 0"
        );

        config
    }
}
