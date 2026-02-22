//! Tools for agents
//!
//! This module provides various tool categories that can be used by agents.
//! Each category is organized into its own submodule.
//!
//! # Available Tool Categories
//!
//! - [`file_system`] - File operations (read, write, edit, list, glob, grep)
//! - [`shell`] - Shell session management (bash, session management)

pub mod file_system;
pub mod shell;

// Re-export commonly used types for convenience
pub use file_system::{
    EditArgs, EditOutput, EditTool, GlobArgs, GlobOutput, GlobTool, GrepArgs, GrepOutput, GrepTool,
    ListArgs, ListOutput, ListTool, ReadArgs, ReadOutput, ReadTool, WriteArgs, WriteOutput,
    WriteTool, file_system_tool_set,
};

pub use shell::{
    BashArgs, BashOutput, BashTool, CaptureOutputArgs, CaptureOutputOutput, CaptureOutputTool,
    CreateSessionArgs, CreateSessionOutput, CreateSessionTool, KillSessionArgs, KillSessionOutput,
    KillSessionTool, ListSessionsArgs, ListSessionsOutput, ListSessionsTool, RestartSessionArgs,
    RestartSessionOutput, RestartSessionTool, SendInputArgs, SendInputOutput, SendInputTool,
    SessionInfo, ShellBackend, ShellError, ShellOutput, SwitchSessionArgs, SwitchSessionOutput,
    SwitchSessionTool, shell_tool_set,
};
