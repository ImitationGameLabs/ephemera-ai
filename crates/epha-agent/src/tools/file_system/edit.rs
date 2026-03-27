use crate::tools::AgentTool;
use crate::tools::file_system::error::EditError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

/// Tool name constant
const NAME: &str = "edit_file";

/// Arguments for the EditTool
#[derive(Deserialize, Serialize, Debug)]
pub struct EditArgs {
    /// The absolute path to the file to edit
    pub file_path: PathBuf,

    /// The text to search for - must match EXACTLY once
    pub old_string: String,

    /// The text to replace it with
    pub new_string: String,

    /// If true, replace all occurrences of old_string
    #[serde(default)]
    pub replace_all: bool,
}

/// Output from the EditTool
#[derive(Debug, Serialize, Deserialize)]
pub struct EditOutput {
    /// The path that was edited
    pub path: PathBuf,
    /// Number of replacements made
    pub replacements: usize,
}

/// Tool for editing files using str_replace style editing
///
/// This tool performs exact string replacement. The old_string must match
/// exactly once (unless replace_all is true). This is safer and more precise
/// than sed-style editing.
pub struct EditTool;

impl EditTool {
    pub fn new() -> Self {
        Self
    }

    /// Count occurrences of a pattern in text
    fn count_occurrences(haystack: &str, needle: &str) -> usize {
        haystack.matches(needle).count()
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentTool for EditTool {
    fn name(&self) -> &str {
        NAME
    }

    fn description(&self) -> &str {
        "Perform exact string replacements in files. Use this tool for editing existing files. The old_string must match EXACTLY once unless replace_all is true. This is safer and more precise than sed-style editing."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The text to search for - must match exactly. Include sufficient context to ensure unique matching."
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace old_string with"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "If true, replace all occurrences of old_string. Use with caution."
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: EditArgs = serde_json::from_str(args_json)?;

        // Check if old and new are the same
        if args.old_string == args.new_string {
            return Err(Box::new(EditError::NoChange));
        }

        // Read the file
        let content = std::fs::read_to_string(&args.file_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                EditError::FileTool(crate::tools::file_system::error::FileToolError::NotFound(
                    args.file_path.clone(),
                ))
            } else {
                EditError::FileTool(crate::tools::file_system::error::FileToolError::io(
                    &args.file_path,
                    e,
                ))
            }
        })?;

        // Count occurrences
        let occurrences = Self::count_occurrences(&content, &args.old_string);

        if occurrences == 0 {
            return Err(Box::new(EditError::NoMatch { file: args.file_path }));
        }

        // Check for multiple matches when not using replace_all
        if !args.replace_all && occurrences > 1 {
            return Err(Box::new(EditError::MultipleMatches {
                file: args.file_path,
                count: occurrences,
            }));
        }

        // Perform replacement
        let new_content = if args.replace_all {
            content.replace(&args.old_string, &args.new_string)
        } else {
            // Safe to replace - we know there's exactly one match
            content.replacen(&args.old_string, &args.new_string, 1)
        };

        // Write back
        std::fs::write(&args.file_path, &new_content).map_err(|e| {
            EditError::FileTool(crate::tools::file_system::error::FileToolError::io(
                &args.file_path,
                e,
            ))
        })?;

        let output = EditOutput { path: args.file_path, replacements: occurrences };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_edit_single_match() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world\n").unwrap();

        let tool = EditTool::new();
        let args = EditArgs {
            file_path: file_path.clone(),
            old_string: "world".to_string(),
            new_string: "Rust".to_string(),
            replace_all: false,
        };

        tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello Rust\n");
    }

    #[tokio::test]
    async fn test_edit_no_match() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world\n").unwrap();

        let tool = EditTool::new();
        let args = EditArgs {
            file_path,
            old_string: "nonexistent".to_string(),
            new_string: "replacement".to_string(),
            replace_all: false,
        };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("NoMatch")
                || err_msg.contains("no match")
                || err_msg.contains("No match")
        );
    }

    #[tokio::test]
    async fn test_edit_multiple_matches_without_replace_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "foo bar foo baz foo\n").unwrap();

        let tool = EditTool::new();
        let args = EditArgs {
            file_path,
            old_string: "foo".to_string(),
            new_string: "qux".to_string(),
            replace_all: false,
        };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("matches") || err_msg.contains("MultipleMatches"));
    }

    #[tokio::test]
    async fn test_edit_multiple_matches_with_replace_all() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "foo bar foo baz foo\n").unwrap();

        let tool = EditTool::new();
        let args = EditArgs {
            file_path: file_path.clone(),
            old_string: "foo".to_string(),
            new_string: "qux".to_string(),
            replace_all: true,
        };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();
        let output: EditOutput = serde_json::from_str(&result).unwrap();
        assert_eq!(output.replacements, 3);

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "qux bar qux baz qux\n");
    }

    #[tokio::test]
    async fn test_edit_no_change() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "hello world\n").unwrap();

        let tool = EditTool::new();
        let args = EditArgs {
            file_path,
            old_string: "world".to_string(),
            new_string: "world".to_string(),
            replace_all: false,
        };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("identical") || err_msg.contains("NoChange"));
    }

    #[tokio::test]
    async fn test_edit_with_multiline_old_string() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        let tool = EditTool::new();
        let args = EditArgs {
            file_path: file_path.clone(),
            old_string: "line 1\nline 2".to_string(),
            new_string: "new line 1\nnew line 2".to_string(),
            replace_all: false,
        };

        tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new line 1\nnew line 2\nline 3\n");
    }
}
