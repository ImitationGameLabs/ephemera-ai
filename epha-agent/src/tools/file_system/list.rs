use crate::tools::file_system::error::FileToolError;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

/// Arguments for the ListTool
#[derive(Deserialize, Debug)]
pub struct ListArgs {
    /// The absolute path to the directory to list
    pub path: PathBuf,
}

/// A single directory entry
#[derive(Debug, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,
}

/// Output from the ListTool
#[derive(Debug, Serialize)]
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

impl Tool for ListTool {
    const NAME: &'static str = "list_directory";

    type Error = FileToolError;
    type Args = ListArgs;
    type Output = ListOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "list_directory",
            "description": "List the contents of a directory. Returns the names, types (file/directory), and sizes of entries. Use this for exploring directory structure.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The absolute path to the directory to list"
                    }
                },
                "required": ["path"]
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
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
            return Err(FileToolError::NotADirectory(args.path));
        }

        // Read directory entries
        let mut entries = Vec::new();
        let read_dir =
            std::fs::read_dir(&args.path).map_err(|e| FileToolError::io(&args.path, e))?;

        for entry in read_dir {
            let entry = entry.map_err(|e| FileToolError::io(&args.path, e))?;

            let name = entry.file_name().to_string_lossy().to_string();
            let metadata = entry
                .metadata()
                .map_err(|e| FileToolError::io(entry.path(), e))?;

            entries.push(DirEntry {
                name,
                is_dir: metadata.is_dir(),
                size: if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                },
            });
        }

        // Sort entries: directories first, then files, alphabetically within each group
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(ListOutput {
            path: args.path,
            entries,
        })
    }
}

impl std::fmt::Display for ListOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Contents of {}:", self.path.display())?;
        for entry in &self.entries {
            if entry.is_dir {
                writeln!(f, "  {}/", entry.name)?;
            } else {
                let size = entry
                    .size
                    .map(|s| format!(" ({} bytes)", s))
                    .unwrap_or_default();
                writeln!(f, "  {}{}", entry.name, size)?;
            }
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
    async fn test_list_directory() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("file1.txt"), "content").unwrap();
        fs::create_dir(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("file2.txt"), "more content").unwrap();

        let tool = ListTool::new();
        let args = ListArgs {
            path: temp_dir.path().to_path_buf(),
        };

        let result = tool.call(args).await.unwrap();
        assert_eq!(result.entries.len(), 3);

        // Check sorting: directories first
        assert!(result.entries[0].is_dir);
        assert_eq!(result.entries[0].name, "subdir");
    }

    #[tokio::test]
    async fn test_list_nonexistent_directory() {
        let tool = ListTool::new();
        let args = ListArgs {
            path: PathBuf::from("/nonexistent/directory"),
        };

        let result = tool.call(args).await;
        assert!(matches!(result, Err(FileToolError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_file_instead_of_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let tool = ListTool::new();
        let args = ListArgs { path: file_path };

        let result = tool.call(args).await;
        assert!(matches!(result, Err(FileToolError::NotADirectory(_))));
    }
}
