use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const ENV_SERVER_URL: &str = "ATRIUM_SERVER_URL";
const ENV_AUTH: &str = "ATRIUM_AUTH";
const ENV_BIO: &str = "ATRIUM_BIO";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct AuthConfig {
    pub username: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Config {
    #[serde(rename = "server-url")]
    pub server_url: String,
    pub auth: Option<AuthConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedConfig {
    pub server_url: String,
    pub auth: AuthConfig,
}

#[derive(Debug, Default, PartialEq)]
struct MissingConfig {
    pub server_url: bool,
    pub auth: bool,
}

impl MissingConfig {
    pub fn is_empty(&self) -> bool {
        !self.server_url && !self.auth
    }

    pub fn to_error_message(&self) -> String {
        let mut lines = vec!["error: missing required configuration".to_string(), String::new()];

        if self.server_url {
            lines.push("  server-url is not configured".to_string());
        }
        if self.auth {
            lines.push("  auth is not configured".to_string());
        }

        lines.push(String::new());
        lines.push("  Run the following to configure:".to_string());

        if self.server_url {
            lines.push("    atrium-cli config set server-url <url>".to_string());
        }
        if self.auth {
            lines.push("    atrium-cli config set auth.username <name>".to_string());
            lines.push("    atrium-cli config set auth.password <password>".to_string());
            lines.push(format!("    or set env: {}=username:password", ENV_AUTH));
        }

        lines.join("\n")
    }
}

pub fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| {
        eprintln!("Warning: Could not determine config directory, using current directory");
        PathBuf::from(".")
    });

    config_dir.join("atrium-cli").join("config.json")
}

pub fn config_file_path() -> String {
    config_path().display().to_string()
}

/// Load configuration from a specific path.
/// Returns default config if the file does not exist.
/// Returns error if the file exists but cannot be read or parsed.
pub fn load_config_from(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}

/// Save configuration to a specific path.
/// Creates parent directories if they don't exist.
pub fn save_config_to(config: &Config, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(path, content)
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
}

/// Load configuration from the default path (~/.config/atrium-cli/config.json)
#[allow(dead_code)]
pub fn load_config() -> Result<Config> {
    load_config_from(&config_path())
}

/// Save configuration to the default path (~/.config/atrium-cli/config.json)
#[allow(dead_code)]
pub fn save_config(config: &Config) -> Result<()> {
    save_config_to(config, &config_path())
}

fn env_server_url() -> Option<String> {
    env::var(ENV_SERVER_URL).ok().filter(|s| !s.is_empty())
}

fn file_server_url(config: &Config) -> Option<String> {
    if config.server_url.is_empty() { None } else { Some(config.server_url.clone()) }
}

fn file_auth(config: &Config) -> Option<AuthConfig> {
    config.auth.as_ref().and_then(|a| {
        if a.username.is_empty() || a.password.is_empty() {
            None
        } else {
            Some(AuthConfig {
                username: a.username.clone(),
                password: a.password.clone(),
                bio: a.bio.clone(),
            })
        }
    })
}

/// Resolve configuration with environment variable override support.
///
/// Priority: environment variables > file configuration
pub fn resolve_config_from(path: &Path) -> Result<ResolvedConfig> {
    let file_config = match load_config_from(path) {
        Ok(config) => config,
        Err(e) => {
            if path.exists() {
                eprintln!(
                    "Warning: Failed to load config file ({}): {}",
                    path.display(),
                    e
                );
            }
            Config::default()
        }
    };

    let server_url = env_server_url().or_else(|| file_server_url(&file_config));

    let auth = if let Some(auth_str) = env::var(ENV_AUTH).ok().filter(|s| !s.is_empty()) {
        let (username, password) = auth_str.split_once(':').ok_or_else(|| {
            anyhow!(
                "ATRIUM_AUTH format is invalid (expected username:password), got: {}",
                auth_str
            )
        })?;
        if username.is_empty() || password.is_empty() {
            return Err(anyhow!(
                "ATRIUM_AUTH format is invalid (expected username:password), got: {}",
                auth_str
            ));
        }
        Some(AuthConfig {
            username: username.to_string(),
            password: password.to_string(),
            bio: env::var(ENV_BIO).ok().filter(|s| !s.is_empty()),
        })
    } else {
        file_auth(&file_config)
    };

    let missing = MissingConfig { server_url: server_url.is_none(), auth: auth.is_none() };
    if !missing.is_empty() {
        return Err(anyhow!("{}", missing.to_error_message()));
    }

    Ok(ResolvedConfig { server_url: server_url.unwrap(), auth: auth.unwrap() })
}

/// Resolve configuration from the default path
pub fn resolve_config() -> Result<ResolvedConfig> {
    resolve_config_from(&config_path())
}

/// Get a configuration value from a specific path
pub fn get_config_value_from(path: &Path, key: &str) -> Result<Option<String>> {
    let config = load_config_from(path)?;

    match key {
        "server-url" => {
            Ok(if config.server_url.is_empty() { None } else { Some(config.server_url) })
        }
        "auth.username" => Ok(config.auth.as_ref().map(|a| a.username.clone())),
        "auth.password" => Ok(config.auth.as_ref().map(|a| a.password.clone())),
        "auth.bio" => Ok(config.auth.as_ref().and_then(|a| a.bio.clone())),
        _ => Err(anyhow!(
            "Unknown config key: {}. Valid keys: server-url, auth.username, auth.password, auth.bio",
            key
        )),
    }
}

/// Set a configuration value at a specific path
/// Creates auth section if setting auth.username or auth.password or auth.bio when it doesn't exist
pub fn set_config_value_to(path: &Path, key: &str, value: &str) -> Result<()> {
    let mut config = load_config_from(path).unwrap_or_default();

    match key {
        "server-url" => {
            config.server_url = value.to_string();
        }
        "auth.username" => {
            let auth = config.auth.get_or_insert_with(AuthConfig::default);
            auth.username = value.to_string();
        }
        "auth.password" => {
            let auth = config.auth.get_or_insert_with(AuthConfig::default);
            auth.password = value.to_string();
        }
        "auth.bio" => {
            let auth = config.auth.get_or_insert_with(AuthConfig::default);
            auth.bio = if value.is_empty() { None } else { Some(value.to_string()) };
        }
        _ => {
            return Err(anyhow!(
                "Unknown config key: {}. Valid keys: server-url, auth.username, auth.password, auth.bio",
                key
            ));
        }
    }

    save_config_to(&config, path)?;
    Ok(())
}

/// Get a configuration value from the default path
pub fn get_config_value(key: &str) -> Result<Option<String>> {
    get_config_value_from(&config_path(), key)
}

/// Set a configuration value at the default path
pub fn set_config_value(key: &str, value: &str) -> Result<()> {
    set_config_value_to(&config_path(), key, value)
}

/// List all configuration values from a specific path
pub fn list_config_from(path: &Path) -> Result<String> {
    let config = load_config_from(path)?;

    let mut output = format!("# Config file: {}\n", path.display());
    output.push_str(&serde_json::to_string_pretty(&config)?);

    Ok(output)
}

/// List all configuration values from the default path
pub fn list_config() -> Result<String> {
    list_config_from(&config_path())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// EnvGuard saves and restores environment variables during tests
    /// to ensure test isolation
    struct EnvGuard {
        server_url: Option<String>,
        auth: Option<String>,
    }

    impl EnvGuard {
        fn new() -> Self {
            // Save current env vars
            let guard =
                Self { server_url: env::var(ENV_SERVER_URL).ok(), auth: env::var(ENV_AUTH).ok() };
            // Clear all env vars for test isolation
            // SAFETY: This is safe in single-threaded tests
            unsafe {
                env::remove_var(ENV_SERVER_URL);
                env::remove_var(ENV_AUTH);
            }
            guard
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // Restore original env vars after test
            // SAFETY: This is safe in single-threaded tests
            unsafe {
                match &self.server_url {
                    Some(v) => env::set_var(ENV_SERVER_URL, v),
                    None => env::remove_var(ENV_SERVER_URL),
                }
                match &self.auth {
                    Some(v) => env::set_var(ENV_AUTH, v),
                    None => env::remove_var(ENV_AUTH),
                }
            }
        }
    }

    /// Creates a temporary directory and config path for testing
    fn setup_temp_config() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("config.json");
        (temp_dir, config_path)
    }

    // ==================== Core I/O Tests ====================

    #[test]
    fn test_save_and_load_config() {
        let (_temp_dir, config_path) = setup_temp_config();

        let config = Config {
            server_url: "http://example.com".to_string(),
            auth: Some(AuthConfig {
                username: "user".to_string(),
                password: "pass".to_string(),
                bio: None,
            }),
        };

        save_config_to(&config, &config_path).unwrap();
        let loaded = load_config_from(&config_path).unwrap();

        assert_eq!(loaded, config);
    }

    #[test]
    fn test_load_nonexistent_config_returns_default() {
        let (_temp_dir, config_path) = setup_temp_config();

        let loaded = load_config_from(&config_path).unwrap();

        assert_eq!(loaded, Config::default());
    }

    #[test]
    fn test_load_invalid_json_returns_error() {
        let (_temp_dir, config_path) = setup_temp_config();

        fs::write(&config_path, "{ invalid json }").unwrap();

        let result = load_config_from(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("subdir/nested/config.json");

        let config = Config::default();
        save_config_to(&config, &config_path).unwrap();

        assert!(config_path.exists());
    }

    // ==================== Config Value Operations Tests ====================

    #[test]
    fn test_set_and_get_server_url() {
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "server-url", "http://example.com").unwrap();
        let value = get_config_value_from(&config_path, "server-url").unwrap();

        assert_eq!(value, Some("http://example.com".to_string()));
    }

    #[test]
    fn test_set_and_get_auth_username() {
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "auth.username", "testuser").unwrap();
        let value = get_config_value_from(&config_path, "auth.username").unwrap();

        assert_eq!(value, Some("testuser".to_string()));
    }

    #[test]
    fn test_set_and_get_auth_password() {
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "auth.password", "testpass").unwrap();
        let value = get_config_value_from(&config_path, "auth.password").unwrap();

        assert_eq!(value, Some("testpass".to_string()));
    }

    #[test]
    fn test_set_auth_creates_auth_if_missing() {
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "auth.username", "user").unwrap();

        let config = load_config_from(&config_path).unwrap();
        assert!(config.auth.is_some());
        assert_eq!(config.auth.unwrap().username, "user");
    }

    #[test]
    fn test_get_unset_value_returns_none() {
        let (_temp_dir, config_path) = setup_temp_config();

        let server_url = get_config_value_from(&config_path, "server-url").unwrap();
        let username = get_config_value_from(&config_path, "auth.username").unwrap();

        assert_eq!(server_url, None);
        assert_eq!(username, None);
    }

    #[test]
    fn test_get_invalid_key_returns_error() {
        let (_temp_dir, config_path) = setup_temp_config();

        let result = get_config_value_from(&config_path, "invalid.key");

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown config key")
        );
    }

    #[test]
    fn test_set_invalid_key_returns_error() {
        let (_temp_dir, config_path) = setup_temp_config();

        let result = set_config_value_to(&config_path, "invalid.key", "value");

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown config key")
        );
    }

    #[test]
    fn test_list_config() {
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "server-url", "http://example.com").unwrap();
        set_config_value_to(&config_path, "auth.username", "user").unwrap();

        let output = list_config_from(&config_path).unwrap();

        assert!(output.contains("server-url"));
        assert!(output.contains("http://example.com"));
        assert!(output.contains("auth"));
        assert!(output.contains("user"));
    }

    // ==================== Environment Variable Override Tests ====================
    // Note: These tests must run serially because they modify global environment variables

    #[test]
    #[serial_test::serial]
    fn test_resolve_from_env_only() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        // SAFETY: Single-threaded test with EnvGuard protection
        unsafe {
            env::set_var(ENV_SERVER_URL, "http://env.example.com");
            env::set_var(ENV_AUTH, "env_user:env_pass");
        }

        let resolved = resolve_config_from(&config_path).unwrap();

        assert_eq!(resolved.server_url, "http://env.example.com");
        assert_eq!(resolved.auth.username, "env_user");
        assert_eq!(resolved.auth.password, "env_pass");
    }

    #[test]
    #[serial_test::serial]
    fn test_resolve_from_file_only() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        let config = Config {
            server_url: "http://file.example.com".to_string(),
            auth: Some(AuthConfig {
                username: "file_user".to_string(),
                password: "file_pass".to_string(),
                bio: None,
            }),
        };
        save_config_to(&config, &config_path).unwrap();

        let resolved = resolve_config_from(&config_path).unwrap();

        assert_eq!(resolved.server_url, "http://file.example.com");
        assert_eq!(resolved.auth.username, "file_user");
        assert_eq!(resolved.auth.password, "file_pass");
    }

    #[test]
    #[serial_test::serial]
    fn test_env_overrides_file() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        let config = Config {
            server_url: "http://file.example.com".to_string(),
            auth: Some(AuthConfig {
                username: "file_user".to_string(),
                password: "file_pass".to_string(),
                bio: None,
            }),
        };
        save_config_to(&config, &config_path).unwrap();

        // SAFETY: Single-threaded test with EnvGuard protection
        unsafe {
            env::set_var(ENV_SERVER_URL, "http://env.example.com");
            env::set_var(ENV_AUTH, "env_user:env_pass");
        }

        let resolved = resolve_config_from(&config_path).unwrap();

        assert_eq!(resolved.server_url, "http://env.example.com");
        assert_eq!(resolved.auth.username, "env_user");
        assert_eq!(resolved.auth.password, "env_pass");
    }

    #[test]
    #[serial_test::serial]
    fn test_partial_env_fallback_to_file() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        let config = Config {
            server_url: "http://file.example.com".to_string(),
            auth: Some(AuthConfig {
                username: "file_user".to_string(),
                password: "file_pass".to_string(),
                bio: None,
            }),
        };
        save_config_to(&config, &config_path).unwrap();

        // SAFETY: Single-threaded test with EnvGuard protection
        unsafe {
            env::set_var(ENV_SERVER_URL, "http://env.example.com");
        }

        let resolved = resolve_config_from(&config_path).unwrap();

        // Env provides server-url, file provides auth
        assert_eq!(resolved.server_url, "http://env.example.com");
        assert_eq!(resolved.auth.username, "file_user");
        assert_eq!(resolved.auth.password, "file_pass");
    }

    #[test]
    #[serial_test::serial]
    fn test_empty_env_ignored() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        let config = Config {
            server_url: "http://file.example.com".to_string(),
            auth: Some(AuthConfig {
                username: "file_user".to_string(),
                password: "file_pass".to_string(),
                bio: None,
            }),
        };
        save_config_to(&config, &config_path).unwrap();

        // SAFETY: Single-threaded test with EnvGuard protection
        unsafe {
            env::set_var(ENV_SERVER_URL, "");
            env::set_var(ENV_AUTH, "");
        }

        let resolved = resolve_config_from(&config_path).unwrap();

        // Empty env vars are ignored, file config is used
        assert_eq!(resolved.server_url, "http://file.example.com");
        assert_eq!(resolved.auth.username, "file_user");
        assert_eq!(resolved.auth.password, "file_pass");
    }

    #[test]
    #[serial_test::serial]
    fn test_empty_env_and_empty_file_fails() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        // SAFETY: Single-threaded test with EnvGuard protection
        unsafe {
            env::set_var(ENV_SERVER_URL, "");
            env::set_var(ENV_AUTH, "");
        }

        let result = resolve_config_from(&config_path);

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("server-url is not configured"));
        assert!(msg.contains("auth is not configured"));
    }

    // ==================== Missing Config Detection Tests ====================

    #[test]
    #[serial_test::serial]
    fn test_missing_server_url() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "auth.username", "user").unwrap();
        set_config_value_to(&config_path, "auth.password", "pass").unwrap();

        let result = resolve_config_from(&config_path);

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("server-url is not configured"));
        assert!(!msg.contains("auth is not configured"));
    }

    #[test]
    #[serial_test::serial]
    fn test_missing_auth() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "server-url", "http://example.com").unwrap();

        let result = resolve_config_from(&config_path);

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(!msg.contains("server-url is not configured"));
        assert!(msg.contains("auth is not configured"));
    }

    #[test]
    #[serial_test::serial]
    fn test_missing_both() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        let result = resolve_config_from(&config_path);

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("server-url is not configured"));
        assert!(msg.contains("auth is not configured"));
    }

    #[test]
    fn test_missing_config_error_message_server_url() {
        let missing = MissingConfig { server_url: true, auth: false };

        let msg = missing.to_error_message();

        assert!(msg.contains("server-url is not configured"));
        assert!(msg.contains("atrium-cli config set server-url"));
        assert!(!msg.contains("auth is not configured"));
    }

    #[test]
    fn test_missing_config_error_message_auth() {
        let missing = MissingConfig { server_url: false, auth: true };

        let msg = missing.to_error_message();

        assert!(msg.contains("auth is not configured"));
        assert!(msg.contains("atrium-cli config set auth.username"));
        assert!(msg.contains("atrium-cli config set auth.password"));
        assert!(msg.contains(ENV_AUTH));
        assert!(!msg.contains("server-url is not configured"));
    }

    #[test]
    fn test_missing_config_error_message_both() {
        let missing = MissingConfig { server_url: true, auth: true };

        let msg = missing.to_error_message();

        assert!(msg.contains("server-url is not configured"));
        assert!(msg.contains("auth is not configured"));
        assert!(msg.contains("atrium-cli config set server-url"));
        assert!(msg.contains("atrium-cli config set auth.username"));
        assert!(msg.contains("atrium-cli config set auth.password"));
    }

    #[test]
    fn test_missing_config_is_empty() {
        assert!(MissingConfig::default().is_empty());
        assert!(!MissingConfig { server_url: true, auth: false }.is_empty());
        assert!(!MissingConfig { server_url: false, auth: true }.is_empty());
        assert!(!MissingConfig { server_url: true, auth: true }.is_empty());
    }

    // ==================== Edge Cases Tests ====================

    #[test]
    fn test_empty_string_server_url_treated_as_missing() {
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "server-url", "").unwrap();

        let value = get_config_value_from(&config_path, "server-url").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    #[serial_test::serial]
    fn test_empty_string_auth_username_treated_as_missing() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "auth.username", "").unwrap();
        set_config_value_to(&config_path, "auth.password", "pass").unwrap();

        let result = resolve_config_from(&config_path);

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("auth is not configured"));
    }

    #[test]
    #[serial_test::serial]
    fn test_empty_string_auth_password_treated_as_missing() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "auth.username", "user").unwrap();
        set_config_value_to(&config_path, "auth.password", "").unwrap();

        let result = resolve_config_from(&config_path);

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("auth is not configured"));
    }

    #[test]
    fn test_whitespace_preserved() {
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "server-url", "   ").unwrap();

        let value = get_config_value_from(&config_path, "server-url").unwrap();
        assert_eq!(value, Some("   ".to_string()));
    }

    #[test]
    fn test_special_characters_in_values() {
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(
            &config_path,
            "server-url",
            "http://example.com?foo=bar&baz=qux",
        )
        .unwrap();
        set_config_value_to(&config_path, "auth.password", "p@ssw0rd!#$%").unwrap();

        let server_url = get_config_value_from(&config_path, "server-url").unwrap();
        let password = get_config_value_from(&config_path, "auth.password").unwrap();

        assert_eq!(
            server_url,
            Some("http://example.com?foo=bar&baz=qux".to_string())
        );
        assert_eq!(password, Some("p@ssw0rd!#$%".to_string()));
    }

    #[test]
    #[serial_test::serial]
    fn test_auth_env_with_colon_in_password() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        // SAFETY: Single-threaded test with EnvGuard protection
        unsafe {
            env::set_var(ENV_SERVER_URL, "http://example.com");
            env::set_var(ENV_AUTH, "user:p@ss:word:with:colons");
        }

        let resolved = resolve_config_from(&config_path).unwrap();
        assert_eq!(resolved.auth.username, "user");
        assert_eq!(resolved.auth.password, "p@ss:word:with:colons");
    }

    #[test]
    #[serial_test::serial]
    fn test_auth_env_malformed_returns_error() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        set_config_value_to(&config_path, "server-url", "http://example.com").unwrap();
        set_config_value_to(&config_path, "auth.username", "user").unwrap();
        set_config_value_to(&config_path, "auth.password", "pass").unwrap();

        // SAFETY: Single-threaded test with EnvGuard protection
        unsafe {
            env::set_var(ENV_AUTH, "nocolonhere");
        }

        let result = resolve_config_from(&config_path);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("ATRIUM_AUTH format is invalid"));
    }

    #[test]
    #[serial_test::serial]
    fn test_auth_env_empty_username_returns_error() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        // SAFETY: Single-threaded test with EnvGuard protection
        unsafe {
            env::set_var(ENV_SERVER_URL, "http://example.com");
            env::set_var(ENV_AUTH, ":password");
        }

        let result = resolve_config_from(&config_path);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("ATRIUM_AUTH format is invalid"));
    }

    #[test]
    #[serial_test::serial]
    fn test_auth_env_empty_password_returns_error() {
        let _guard = EnvGuard::new();
        let (_temp_dir, config_path) = setup_temp_config();

        // SAFETY: Single-threaded test with EnvGuard protection
        unsafe {
            env::set_var(ENV_SERVER_URL, "http://example.com");
            env::set_var(ENV_AUTH, "username:");
        }

        let result = resolve_config_from(&config_path);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("ATRIUM_AUTH format is invalid"));
    }

    #[test]
    fn test_config_with_extra_json_fields() {
        let (_temp_dir, config_path) = setup_temp_config();

        // JSON with extra fields should be loaded successfully (extra fields ignored)
        let json_with_extra = r#"{
  "server-url": "http://example.com",
  "extra_field": "ignored",
  "auth": {
    "username": "user",
    "password": "pass",
    "extra_nested": 123
  }
}"#;

        fs::write(&config_path, json_with_extra).unwrap();

        let config = load_config_from(&config_path).unwrap();

        assert_eq!(config.server_url, "http://example.com");
        assert_eq!(config.auth.unwrap().username, "user");
    }
}
