use crate::tools::file_system::error::FileToolError;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

/// Arguments for the WriteTool
#[derive(Deserialize, Debug)]
pub struct WriteArgs {
    /// The absolute path to the file to write
    pub file_path: PathBuf,

    /// The content to write to the file
    pub content: String,
}

/// Output from the WriteTool
#[derive(Debug, Serialize)]
pub struct WriteOutput {
    /// The path that was written to
    pub path: PathBuf,
    /// Number of bytes written
    pub bytes_written: usize,
}

/// Tool for writing content to files
///
/// This tool will overwrite existing files. For existing files, you should
/// read them first to understand their contents before overwriting.
pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }

    /// Ensure parent directories exist
    fn ensure_parent_dirs(path: &std::path::Path) -> Result<(), FileToolError> {
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            std::fs::create_dir_all(parent).map_err(|e| FileToolError::io(parent, e))?;
        }
        Ok(())
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for WriteTool {
    const NAME: &'static str = "write_file";

    type Error = FileToolError;
    type Args = WriteArgs;
    type Output = WriteOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "write_file",
            "description": "Write a file to the local filesystem. This tool will OVERWRITE any existing file. For existing files, you must read them first. Use this tool when you need to create new files or completely replace file contents.",
            "parameters": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to write (not a relative path)"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file. ALWAYS provide the complete content."
                    }
                },
                "required": ["file_path", "content"]
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Ensure parent directories exist
        Self::ensure_parent_dirs(&args.file_path)?;

        // Write the content
        let bytes = args.content.len();
        std::fs::write(&args.file_path, &args.content)
            .map_err(|e| FileToolError::io(&args.file_path, e))?;

        Ok(WriteOutput {
            path: args.file_path,
            bytes_written: bytes,
        })
    }
}

impl std::fmt::Display for WriteOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Successfully wrote {} bytes to {}",
            self.bytes_written,
            self.path.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_write_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("new_file.txt");

        let tool = WriteTool::new();
        let args = WriteArgs {
            file_path: file_path.clone(),
            content: "Hello, World!".to_string(),
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result.bytes_written, 13);

        // Verify content was written
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_overwrite_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.txt");
        fs::write(&file_path, "Old content").unwrap();

        let tool = WriteTool::new();
        let args = WriteArgs {
            file_path: file_path.clone(),
            content: "New content".to_string(),
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result.bytes_written, 11);

        // Verify content was overwritten
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "New content");
    }

    #[tokio::test]
    async fn test_write_creates_parent_dirs() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nested/deep/dir/file.txt");

        let tool = WriteTool::new();
        let args = WriteArgs {
            file_path: file_path.clone(),
            content: "Nested content".to_string(),
        };

        let result = tool.call(args).await.unwrap();
        assert!(file_path.exists());
        assert_eq!(result.bytes_written, 14);
    }
}
