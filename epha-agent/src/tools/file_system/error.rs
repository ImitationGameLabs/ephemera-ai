use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during file tool operations
#[derive(Debug, Error)]
pub enum FileToolError {
    #[error("File not found: '{0}'")]
    NotFound(PathBuf),

    #[error("Permission denied: '{0}'")]
    PermissionDenied(PathBuf),

    #[error("IO error at '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Path is not a file: '{0}'")]
    NotAFile(PathBuf),

    #[error("Path is not a directory: '{0}'")]
    NotADirectory(PathBuf),

    #[error("Invalid path: '{0}'")]
    InvalidPath(String),

    #[error(
        "File too large: '{path}' ({size} bytes, max {max} bytes). Use offset and limit to read in chunks."
    )]
    FileTooLarge { path: PathBuf, size: u64, max: u64 },

    #[error("Binary file, cannot read as text: '{0}'")]
    BinaryFile(PathBuf),
}

/// Errors specific to the EditTool (str_replace operations)
#[derive(Debug, Error)]
pub enum EditError {
    #[error("No match found for old_string in '{file}'")]
    NoMatch { file: PathBuf },

    #[error(
        "Found {count} matches in '{file}', expected exactly 1. Use replace_all=true or include more context in old_string."
    )]
    MultipleMatches { file: PathBuf, count: usize },

    #[error("old_string and new_string are identical")]
    NoChange,

    #[error(transparent)]
    FileTool(#[from] FileToolError),
}

/// Errors specific to the GrepTool
#[derive(Debug, Error)]
pub enum GrepError {
    #[error("Invalid regex pattern '{pattern}': {reason}")]
    InvalidPattern { pattern: String, reason: String },

    #[error("Search exceeded maximum depth of {0}")]
    MaxDepthExceeded(usize),

    #[error("Search timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error(transparent)]
    FileTool(#[from] FileToolError),
}

/// Errors specific to the GlobTool
#[derive(Debug, Error)]
pub enum GlobError {
    #[error("Invalid glob pattern '{pattern}': {reason}")]
    InvalidPattern { pattern: String, reason: String },

    #[error("Path does not exist: '{0}'")]
    PathNotFound(PathBuf),

    #[error(transparent)]
    FileTool(#[from] FileToolError),
}

impl FileToolError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        let path = path.into();
        match source.kind() {
            std::io::ErrorKind::NotFound => FileToolError::NotFound(path),
            std::io::ErrorKind::PermissionDenied => FileToolError::PermissionDenied(path),
            _ => FileToolError::Io { path, source },
        }
    }
}
