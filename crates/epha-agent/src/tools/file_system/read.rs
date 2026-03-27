use crate::tools::AgentTool;
use crate::tools::file_system::error::FileToolError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

/// Maximum file size to read (10 MB)
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Default maximum lines to return
const DEFAULT_MAX_LINES: usize = 2000;

/// Tool name constant
const NAME: &str = "read_file";

/// Arguments for the ReadTool
#[derive(Deserialize, Serialize, Debug)]
pub struct ReadArgs {
    /// The absolute path to the file to read
    pub file_path: PathBuf,

    /// Optional line offset to start reading from
    #[serde(default)]
    pub offset: Option<usize>,

    /// Optional limit on number of lines to read
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Output from the ReadTool
#[derive(Debug, Serialize, Deserialize)]
pub struct ReadOutput {
    /// The file content with line numbers (cat -n style)
    pub content: String,
    /// Total lines in the file
    pub total_lines: usize,
    /// Whether the file was truncated
    pub truncated: bool,
}

/// Tool for reading file contents with line numbers
pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }

    /// Check if content appears to be binary
    fn is_binary(content: &[u8]) -> bool {
        // Check for null bytes which indicate binary content
        content.iter().take(8192).any(|&b| b == 0)
    }

    /// Format content with line numbers (cat -n style)
    fn format_with_line_numbers(lines: &[&str], offset: usize) -> String {
        let line_number_width = (offset + lines.len()).to_string().len().max(1);

        lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                format!("{:>width$}\t{}", offset + i + 1, line, width = line_number_width)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentTool for ReadTool {
    fn name(&self) -> &str {
        NAME
    }

    fn description(&self) -> &str {
        "Read a file from the local filesystem. Returns file contents with line numbers (cat -n style). You can access any file directly by using this tool."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "The absolute path to the file to read (not a relative path)"
                },
                "offset": {
                    "type": "integer",
                    "description": "The line number to start reading from (1-indexed). Only provide if the file is too large to read at once."
                },
                "limit": {
                    "type": "integer",
                    "description": "The number of lines to read. Only provide if the file is too large to read at once."
                }
            },
            "required": ["file_path"]
        })
    }

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: ReadArgs = serde_json::from_str(args_json)?;

        // Validate path exists and is a file
        let metadata = std::fs::metadata(&args.file_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileToolError::NotFound(args.file_path.clone())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FileToolError::PermissionDenied(args.file_path.clone())
            } else {
                FileToolError::io(&args.file_path, e)
            }
        })?;

        if !metadata.is_file() {
            return Err(Box::new(FileToolError::NotAFile(args.file_path)));
        }

        // Check file size
        let file_size = metadata.len();
        if file_size > MAX_FILE_SIZE {
            return Err(Box::new(FileToolError::FileTooLarge {
                path: args.file_path,
                size: file_size,
                max: MAX_FILE_SIZE,
            }));
        }

        // Read file content
        let content =
            std::fs::read(&args.file_path).map_err(|e| FileToolError::io(&args.file_path, e))?;

        // Check for binary content
        if Self::is_binary(&content) {
            return Err(Box::new(FileToolError::BinaryFile(args.file_path)));
        }

        // Convert to string
        let text = String::from_utf8_lossy(&content);
        let all_lines: Vec<&str> = text.lines().collect();
        let total_lines = all_lines.len();

        // Apply offset and limit
        let offset = args.offset.unwrap_or(0);
        let limit = args.limit.unwrap_or(DEFAULT_MAX_LINES);

        // Ensure offset is valid
        let offset = offset.min(total_lines);

        // Get the slice of lines we want
        let end = (offset + limit).min(total_lines);
        let selected_lines = &all_lines[offset..end];
        let truncated = end < total_lines;

        // Format with line numbers
        let formatted = Self::format_with_line_numbers(selected_lines, offset);

        let output = ReadOutput { content: formatted, total_lines, truncated };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

        let tool = ReadTool::new();
        let args = ReadArgs { file_path: file_path.clone(), offset: None, limit: None };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();
        let output: ReadOutput = serde_json::from_str(&result).unwrap();
        assert!(output.content.contains("line 1"));
        assert!(output.content.contains("line 2"));
        assert_eq!(output.total_lines, 3);
        assert!(!output.truncated);
    }

    #[tokio::test]
    async fn test_read_file_with_offset_and_limit() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "line 1\nline 2\nline 3\nline 4\nline 5\n").unwrap();

        let tool = ReadTool::new();
        let args = ReadArgs {
            file_path,
            offset: Some(1), // Start from line 2 (0-indexed as 1)
            limit: Some(2),  // Read 2 lines
        };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();
        let output: ReadOutput = serde_json::from_str(&result).unwrap();
        assert!(output.content.contains("line 2"));
        assert!(output.content.contains("line 3"));
        assert!(!output.content.contains("line 1"));
        assert!(!output.content.contains("line 4"));
        assert_eq!(output.total_lines, 5);
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = ReadTool::new();
        let args = ReadArgs {
            file_path: PathBuf::from("/nonexistent/path.txt"),
            offset: None,
            limit: None,
        };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not found") || err_msg.contains("NotFound"));
    }

    #[tokio::test]
    async fn test_read_directory() {
        let temp_dir = TempDir::new().unwrap();

        let tool = ReadTool::new();
        let args = ReadArgs { file_path: temp_dir.path().to_path_buf(), offset: None, limit: None };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not a file") || err_msg.contains("NotAFile"));
    }

    #[tokio::test]
    async fn test_read_binary_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("binary.bin");
        // Write binary content with null bytes
        fs::write(&file_path, b"\x00\x01\x02\x03binary").unwrap();

        let tool = ReadTool::new();
        let args = ReadArgs { file_path, offset: None, limit: None };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("binary") || err_msg.contains("BinaryFile"));
    }
}
