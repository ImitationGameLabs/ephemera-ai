use crate::tools::AgentTool;
use crate::tools::file_system::error::FileToolError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

/// Tool name constant
const NAME: &str = "list_directory";

/// Arguments for the ListTool
#[derive(Deserialize, Serialize, Debug)]
pub struct ListArgs {
    /// The absolute path to the directory to list
    pub path: PathBuf,
}

/// A single directory entry
#[derive(Debug, Serialize, Deserialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

/// Output from the ListTool
#[derive(Debug, Serialize, Deserialize)]
pub struct ListOutput {
    /// The path that was listed
    pub path: PathBuf,
    /// Directory entries
    pub entries: Vec<DirEntry>,
}

/// Tool for listing directory contents
pub struct ListTool;

impl ListTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ListTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentTool for ListTool {
    fn name(&self) -> &str {
        NAME
    }

    fn description(&self) -> &str {
        "List the contents of a directory. Returns the names, types (file/directory), and sizes of entries. Use this for exploring directory structure."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the directory to list"
                }
            },
            "required": ["path"]
        })
    }

    async fn call(
        &self,
        args_json: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let args: ListArgs = serde_json::from_str(args_json)?;

        // Validate path exists and is a directory
        let metadata = std::fs::metadata(&args.path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FileToolError::NotFound(args.path.clone())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                FileToolError::PermissionDenied(args.path.clone())
            } else {
                FileToolError::io(&args.path, e)
            }
        })?;

        if !metadata.is_dir() {
            return Err(Box::new(FileToolError::NotADirectory(args.path)));
        }

        // Read directory entries
        let mut entries = Vec::new();
        let read_dir =
            std::fs::read_dir(&args.path).map_err(|e| FileToolError::io(&args.path, e))?;

        for entry in read_dir {
            let entry = entry.map_err(|e| FileToolError::io(&args.path, e))?;

            let name = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata().map_err(|e| FileToolError::io(entry.path(), e))?;

            entries.push(DirEntry {
                name,
                is_dir: metadata.is_dir(),
                size: if metadata.is_file() { Some(metadata.len()) } else { None },
            });
        }

        // Sort entries: directories first, then files, alphabetically within each group
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        let output = ListOutput { path: args.path, entries };

        Ok(serde_json::to_string(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_list_directory() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "more content").unwrap();

        let tool = ListTool::new();
        let args = ListArgs { path: temp_dir.path().to_path_buf() };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await.unwrap();
        let output: ListOutput = serde_json::from_str(&result).unwrap();
        assert_eq!(output.entries.len(), 3);

        // Check sorting: directories first
        assert!(output.entries[0].is_dir);
        assert_eq!(output.entries[0].name, "subdir");
    }

    #[tokio::test]
    async fn test_list_nonexistent_directory() {
        let tool = ListTool::new();
        let args = ListArgs { path: PathBuf::from("/nonexistent/directory") };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not found") || err_msg.contains("NotFound"));
    }

    #[tokio::test]
    async fn test_list_file_instead_of_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let tool = ListTool::new();
        let args = ListArgs { path: file_path };

        let result = tool.call(&serde_json::to_string(&args).unwrap()).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("not a directory") || err_msg.contains("NotADirectory"));
    }
}
