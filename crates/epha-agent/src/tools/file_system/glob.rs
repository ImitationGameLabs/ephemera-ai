use crate::tools::AgentTool;
use crate::tools::file_system::error::GlobError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

/// Tool name constant
const NAME: &str = "glob_search";

/// Default directories to ignore during glob search
const DEFAULT_IGNORE_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    ".svn",
    ".hg",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".idea",
    ".vscode",
];

/// Arguments for the GlobTool
#[derive(Deserialize, Serialize, Debug)]
pub struct GlobArgs {
    /// The glob pattern to match files against (e.g., "**/*.rs", "src/**/*.ts")
    pub pattern: String,

    /// The directory to search in (defaults to current directory)
    #[serde(default)]
    pub path: Option<PathBuf>,
}

/// Output from the GlobTool
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobOutput {
    /// The pattern that was searched
    pub pattern: String,
    /// The base path that was searched
    pub base_path: PathBuf,
    /// Matching file paths, sorted by modification time (newest first)
    pub matches: Vec<PathBuf>,
}

/// Tool for finding files using glob patterns
pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }

    /// Check if a path should be ignored
    fn should_ignore(path: &std::path::Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| DEFAULT_IGNORE_DIRS.contains(&name))
            .unwrap_or(false)
    }

    /// Recursively walk directory and collect matches
    fn collect_matches(
        dir: &std::path::Path,
        pattern: &glob::Pattern,
        results: &mut Vec<PathBuf>,
    ) -> Result<(), GlobError> {
        let entries = std::fs::read_dir(dir).map_err(|e| {
            GlobError::FileTool(crate::tools::file_system::error::FileToolError::io(dir, e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                GlobError::FileTool(crate::tools::file_system::error::FileToolError::io(dir, e))
            })?;
            let path = entry.path();

            // Skip ignored directories
            if Self::should_ignore(&path) {
                continue;
            }

            if path.is_dir() {
                Self::collect_matches(&path, pattern, results)?;
            } else if pattern.matches_path(&path) {
                results.push(path);
            }
        }

        Ok(())
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentTool for GlobTool {
    fn name(&self) -> &str {
        NAME
    }

    fn description(&self) -> &str {
        "Fast file pattern matching tool that works with any codebase size. Supports glob patterns like '**/*.js' or 'src/**/*.ts'. Returns matching file paths sorted by modification time."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match files against (e.g., '**/*.rs', 'src/**/*.ts', '*.json')"
                },
                "path": {
                    "type": "string",
                    "description": "The directory to search in. Defaults to current working directory."
                }
            },
            "required": ["pattern"]
        })
    }

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: GlobArgs = serde_json::from_str(args_json)?;

        let base_path = args.path.unwrap_or_else(|| std::env::current_dir().unwrap());

        // Validate base path exists
        if !base_path.exists() {
            return Err(Box::new(GlobError::PathNotFound(base_path)));
        }

        // Parse the glob pattern
        let pattern = glob::Pattern::new(&args.pattern).map_err(|e| GlobError::InvalidPattern {
            pattern: args.pattern.clone(),
            reason: e.to_string(),
        })?;

        // Collect all matches
        let mut matches = Vec::new();
        Self::collect_matches(&base_path, &pattern, &mut matches)?;

        // Sort by modification time (newest first)
        matches.sort_by(|a, b| {
            let a_time = std::fs::metadata(a)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let b_time = std::fs::metadata(b)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            b_time.cmp(&a_time)
        });

        let output = GlobOutput { pattern: args.pattern, base_path, matches };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_glob_find_rs_files() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("main.rs"), "").unwrap();
        fs::write(temp_dir.path().join("lib.rs"), "").unwrap();
        fs::write(temp_dir.path().join("test.txt"), "").unwrap();
        fs::create_dir(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/mod.rs"), "").unwrap();

        let tool = GlobTool::new();
        let args =
            GlobArgs { pattern: "**/*.rs".to_string(), path: Some(temp_dir.path().to_path_buf()) };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();
        let output: GlobOutput = serde_json::from_str(&result).unwrap();
        assert_eq!(output.matches.len(), 3);
    }

    #[tokio::test]
    async fn test_glob_ignores_node_modules() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("index.js"), "").unwrap();
        fs::create_dir(temp_dir.path().join("node_modules")).unwrap();
        fs::write(temp_dir.path().join("node_modules/package.js"), "").unwrap();

        let tool = GlobTool::new();
        let args =
            GlobArgs { pattern: "**/*.js".to_string(), path: Some(temp_dir.path().to_path_buf()) };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();
        let output: GlobOutput = serde_json::from_str(&result).unwrap();
        assert_eq!(output.matches.len(), 1);
        assert!(output.matches[0].ends_with("index.js"));
    }

    #[tokio::test]
    async fn test_glob_invalid_pattern() {
        let tool = GlobTool::new();
        let args = GlobArgs { pattern: "[invalid".to_string(), path: None };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Invalid glob pattern")
                || err_msg.contains("invalid pattern")
                || err_msg.contains("InvalidPattern")
        );
    }
}
