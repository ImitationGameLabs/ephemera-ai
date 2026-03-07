use crate::tools::file_system::error::GrepError;
use regex::Regex;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

/// Maximum search depth to prevent infinite recursion
const MAX_DEPTH: usize = 20;

/// Default directories to ignore during grep search
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

/// Default file patterns to ignore
const DEFAULT_IGNORE_FILES: &[&str] = &[
    "*.lock",
    "*.min.js",
    "*.min.css",
    "package-lock.json",
    "yarn.lock",
    "Cargo.lock",
];

/// Maximum total matches to return
const MAX_MATCHES: usize = 1000;

/// Arguments for the GrepTool
#[derive(Deserialize, Debug)]
pub struct GrepArgs {
    /// The regular expression pattern to search for
    pub pattern: String,

    /// The directory or file to search in (defaults to current directory)
    #[serde(default)]
    pub path: Option<PathBuf>,

    /// Filter files by glob pattern (e.g., "*.rs", "*.{js,ts}")
    #[serde(default)]
    pub glob: Option<String>,

    /// Output mode: "content" shows matching lines, "files_with_matches" shows only file paths
    #[serde(default = "default_output_mode")]
    pub output_mode: String,

    /// Case insensitive search
    #[serde(default)]
    pub case_insensitive: bool,

    /// Show N lines before each match
    #[serde(default)]
    pub before_context: Option<usize>,

    /// Show N lines after each match
    #[serde(default)]
    pub after_context: Option<usize>,
}

fn default_output_mode() -> String {
    "content".to_string()
}

/// A single match result
#[derive(Debug, Clone, Serialize)]
pub struct GrepMatch {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

/// Output from the GrepTool
#[derive(Debug, Serialize)]
pub struct GrepOutput {
    /// The pattern that was searched
    pub pattern: String,
    /// The base path that was searched
    pub base_path: PathBuf,
    /// Output mode used
    pub output_mode: String,
    /// Matching results
    pub matches: Vec<GrepMatch>,
    /// Whether results were truncated due to MAX_MATCHES
    pub truncated: bool,
}

/// Tool for searching file contents using regular expressions
pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }

    /// Check if a path should be ignored
    fn should_ignore_dir(path: &std::path::Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| DEFAULT_IGNORE_DIRS.contains(&name))
            .unwrap_or(false)
    }

    /// Check if a file should be ignored based on patterns
    fn should_ignore_file(path: &std::path::Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        DEFAULT_IGNORE_FILES.iter().any(|pattern| {
            if pattern.starts_with("*.") {
                let ext = pattern.strip_prefix('*').unwrap();
                file_name.ends_with(ext)
            } else {
                file_name == *pattern
            }
        })
    }

    /// Check if a file matches the glob filter
    fn matches_glob(path: &std::path::Path, glob_pattern: &Option<String>) -> bool {
        match glob_pattern {
            None => true,
            Some(pattern) => {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                // Simple glob matching for patterns like "*.rs" or "*.{js,ts}"
                if pattern.starts_with("*.") {
                    let ext_part = pattern.strip_prefix('*').unwrap();
                    if ext_part.starts_with('{') && ext_part.ends_with('}') {
                        // Handle *.{js,ts} patterns
                        let extensions: Vec<&str> =
                            ext_part[1..ext_part.len() - 1].split(',').collect();
                        extensions.iter().any(|ext| file_name.ends_with(ext))
                    } else {
                        file_name.ends_with(ext_part)
                    }
                } else {
                    file_name == *pattern
                }
            }
        }
    }

    /// Check if content is binary
    fn is_binary(content: &[u8]) -> bool {
        content.iter().take(8192).any(|&b| b == 0)
    }

    /// Search a single file
    fn search_file(
        path: &std::path::Path,
        regex: &Regex,
        args: &GrepArgs,
        results: &mut Vec<GrepMatch>,
        total_matches: &mut usize,
    ) -> Result<(), GrepError> {
        if *total_matches >= MAX_MATCHES {
            return Ok(());
        }

        // Skip ignored files
        if Self::should_ignore_file(path) {
            return Ok(());
        }

        // Check glob filter
        if !Self::matches_glob(path, &args.glob) {
            return Ok(());
        }

        // Read file content
        let content = std::fs::read(path).map_err(|e| {
            GrepError::FileTool(crate::tools::file_system::error::FileToolError::io(path, e))
        })?;

        // Skip binary files
        if Self::is_binary(&content) {
            return Ok(());
        }

        let text = String::from_utf8_lossy(&content);
        let lines: Vec<&str> = text.lines().collect();

        // Search each line
        for (line_idx, line) in lines.iter().enumerate() {
            if regex.is_match(line) {
                let context_before: Vec<String> = args
                    .before_context
                    .map(|n| {
                        (0..n)
                            .rev()
                            .filter_map(|i| {
                                if line_idx > i {
                                    Some(lines[line_idx - i - 1].to_string())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                let context_after: Vec<String> = args
                    .after_context
                    .map(|n| {
                        (1..=n)
                            .filter_map(|i| lines.get(line_idx + i).map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                results.push(GrepMatch {
                    file_path: path.to_path_buf(),
                    line_number: line_idx + 1,
                    line_content: line.to_string(),
                    context_before,
                    context_after,
                });

                *total_matches += 1;
                if *total_matches >= MAX_MATCHES {
                    break;
                }
            }
        }

        Ok(())
    }

    /// Recursively walk directory and search files
    fn search_dir(
        dir: &std::path::Path,
        regex: &Regex,
        args: &GrepArgs,
        results: &mut Vec<GrepMatch>,
        total_matches: &mut usize,
        depth: usize,
    ) -> Result<(), GrepError> {
        if depth > MAX_DEPTH {
            return Err(GrepError::MaxDepthExceeded(MAX_DEPTH));
        }

        if *total_matches >= MAX_MATCHES {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir).map_err(|e| {
            GrepError::FileTool(crate::tools::file_system::error::FileToolError::io(dir, e))
        })?;

        for entry in entries {
            if *total_matches >= MAX_MATCHES {
                break;
            }

            let entry = entry.map_err(|e| {
                GrepError::FileTool(crate::tools::file_system::error::FileToolError::io(dir, e))
            })?;
            let path = entry.path();

            if path.is_dir() {
                // Skip ignored directories
                if Self::should_ignore_dir(&path) {
                    continue;
                }
                Self::search_dir(&path, regex, args, results, total_matches, depth + 1)?;
            } else if path.is_file() {
                Self::search_file(&path, regex, args, results, total_matches)?;
            }
        }

        Ok(())
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for GrepTool {
    const NAME: &'static str = "grep_search";

    type Error = GrepError;
    type Args = GrepArgs;
    type Output = GrepOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "grep_search",
            "description": "A powerful search tool built on regex for searching file contents. Supports full regex syntax. Use this when you need to search within files rather than just finding files.",
            "parameters": {
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regular expression pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory or file to search in. Defaults to current directory."
                    },
                    "glob": {
                        "type": "string",
                        "description": "Glob pattern to filter files (e.g., '*.rs', '*.{js,ts}')"
                    },
                    "output_mode": {
                        "type": "string",
                        "enum": ["content", "files_with_matches"],
                        "description": "Output mode: 'content' shows matching lines, 'files_with_matches' shows only file paths"
                    },
                    "case_insensitive": {
                        "type": "boolean",
                        "description": "Case insensitive search"
                    },
                    "before_context": {
                        "type": "integer",
                        "description": "Show N lines before each match"
                    },
                    "after_context": {
                        "type": "integer",
                        "description": "Show N lines after each match"
                    }
                },
                "required": ["pattern"]
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Build regex
        let mut regex_builder = regex::RegexBuilder::new(&args.pattern);
        regex_builder.case_insensitive(args.case_insensitive);
        let regex = regex_builder
            .build()
            .map_err(|e| GrepError::InvalidPattern {
                pattern: args.pattern.clone(),
                reason: e.to_string(),
            })?;

        // Determine base path
        let base_path = args
            .path
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap());

        // Validate base path exists
        if !base_path.exists() {
            return Err(GrepError::FileTool(
                crate::tools::file_system::error::FileToolError::NotFound(base_path),
            ));
        }

        let mut matches = Vec::new();
        let mut total_matches = 0;

        if base_path.is_file() {
            Self::search_file(&base_path, &regex, &args, &mut matches, &mut total_matches)?;
        } else {
            Self::search_dir(
                &base_path,
                &regex,
                &args,
                &mut matches,
                &mut total_matches,
                0,
            )?;
        }

        let truncated = total_matches >= MAX_MATCHES;

        Ok(GrepOutput {
            pattern: args.pattern,
            base_path,
            output_mode: args.output_mode.clone(),
            matches,
            truncated,
        })
    }
}

impl std::fmt::Display for GrepOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.matches.is_empty() {
            writeln!(
                f,
                "No matches found for pattern '{}' in {}",
                self.pattern,
                self.base_path.display()
            )?;
            return Ok(());
        }

        if self.output_mode == "files_with_matches" {
            // Group by file
            let mut files: std::collections::HashSet<&PathBuf> = std::collections::HashSet::new();
            for m in &self.matches {
                files.insert(&m.file_path);
            }
            writeln!(
                f,
                "Found {} matches in {} file(s):",
                self.matches.len(),
                files.len()
            )?;
            for file in files {
                writeln!(f, "  {}", file.display())?;
            }
        } else {
            writeln!(
                f,
                "Found {} match(es) for '{}':",
                self.matches.len(),
                self.pattern
            )?;
            for m in &self.matches {
                writeln!(f, "\n{}:{}", m.file_path.display(), m.line_number)?;
                for ctx in &m.context_before {
                    writeln!(f, "  {}", ctx)?;
                }
                writeln!(f, "  {}", m.line_content)?;
                for ctx in &m.context_after {
                    writeln!(f, "  {}", ctx)?;
                }
            }
        }

        if self.truncated {
            writeln!(f, "\n(Results truncated at {} matches)", MAX_MATCHES)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_grep_search_content() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("test.rs"),
            "fn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();

        let tool = GrepTool::new();
        let args = GrepArgs {
            pattern: "println".to_string(),
            path: Some(temp_dir.path().to_path_buf()),
            glob: None,
            output_mode: "content".to_string(),
            case_insensitive: false,
            before_context: None,
            after_context: None,
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].line_number, 2);
    }

    #[tokio::test]
    async fn test_grep_search_with_glob() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();
        fs::write(temp_dir.path().join("test.txt"), "fn main() {}").unwrap();

        let tool = GrepTool::new();
        let args = GrepArgs {
            pattern: "main".to_string(),
            path: Some(temp_dir.path().to_path_buf()),
            glob: Some("*.rs".to_string()),
            output_mode: "content".to_string(),
            case_insensitive: false,
            before_context: None,
            after_context: None,
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result.matches.len(), 1);
        assert!(result.matches[0].file_path.ends_with("test.rs"));
    }

    #[tokio::test]
    async fn test_grep_invalid_pattern() {
        let tool = GrepTool::new();
        let args = GrepArgs {
            pattern: "(invalid".to_string(),
            path: None,
            glob: None,
            output_mode: "content".to_string(),
            case_insensitive: false,
            before_context: None,
            after_context: None,
        };

        let result = tool.call(args).await;
        assert!(matches!(result, Err(GrepError::InvalidPattern { .. })));
    }

    #[tokio::test]
    async fn test_grep_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.txt"), "Hello World\n").unwrap();

        let tool = GrepTool::new();
        let args = GrepArgs {
            pattern: "HELLO".to_string(),
            path: Some(temp_dir.path().to_path_buf()),
            glob: None,
            output_mode: "content".to_string(),
            case_insensitive: true,
            before_context: None,
            after_context: None,
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result.matches.len(), 1);
    }

    #[tokio::test]
    async fn test_grep_context() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("test.txt"),
            "line 1\nline 2\nline 3\nmatch here\nline 5\nline 6\n",
        )
        .unwrap();

        let tool = GrepTool::new();
        let args = GrepArgs {
            pattern: "match".to_string(),
            path: Some(temp_dir.path().to_path_buf()),
            glob: None,
            output_mode: "content".to_string(),
            case_insensitive: false,
            before_context: Some(2),
            after_context: Some(2),
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].context_before.len(), 2);
        assert_eq!(result.matches[0].context_after.len(), 2);
    }
}
