//! File system tools for agents
//!
//! This module provides a set of file operation tools that follow Claude Code's
//! design philosophy: use specialized tools for understanding and editing, rather
//! than simple command-line wrappers.
//!
//! # Available Tools
//!
//! - [`ReadTool`] - Read file contents with line numbers (like `cat -n`)
//! - [`WriteTool`] - Write/create files (like `> file`)
//! - [`EditTool`] - str_replace style editing (safer than sed)
//! - [`ListTool`] - List directory contents (like `ls`)
//! - [`GlobTool`] - Pattern matching file search (like `find`/`glob`)
//! - [`GrepTool`] - Regex search in file contents (like `grep`/`rg`)
//!
//! # Example
//!
//! ```rust,ignore
//! use epha_agent::tools::file_system::{ReadTool, WriteTool, EditTool, file_system_tool_set};
//! use epha_agent::tools::AgentTool;
//!
//! // Get all file system tools as a vector
//! let tools = file_system_tool_set();
//!
//! // Or use individual tools
//! let read_tool = ReadTool::new();
//! let write_tool = WriteTool::new();
//! let edit_tool = EditTool::new();
//! ```

mod edit;
mod error;
mod glob;
mod grep;
mod list;
mod read;
mod write;

pub use edit::{EditArgs, EditOutput, EditTool};
pub use error::{EditError, FileToolError, GlobError, GrepError};
pub use glob::{GlobArgs, GlobOutput, GlobTool};
pub use grep::{GrepArgs, GrepMatch, GrepOutput, GrepTool};
pub use list::{DirEntry, ListArgs, ListOutput, ListTool};
pub use read::{ReadArgs, ReadOutput, ReadTool};
pub use write::{WriteArgs, WriteOutput, WriteTool};

use crate::tools::AgentTool;

/// Create a set of all file system tools
///
/// Returns a vector of boxed tools that implement the AgentTool trait.
pub fn file_system_tool_set() -> Vec<Box<dyn AgentTool>> {
    vec![
        Box::new(ReadTool::new()),
        Box::new(WriteTool::new()),
        Box::new(EditTool::new()),
        Box::new(ListTool::new()),
        Box::new(GlobTool::new()),
        Box::new(GrepTool::new()),
    ]
}
