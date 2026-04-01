//! Tools for agents
//!
//! This module provides various tool categories that can be used by agents.
//! Each category is organized into its own submodule.
//!
//! # Available Tool Categories
//!
//! - [`shell`] - Shell session management (bash, session management)

pub mod agent_tool;
pub mod shell;

// Re-export the AgentTool trait for downstream crates
pub use agent_tool::AgentTool;

pub use shell::{
    BashArgs, BashOutput, BashTool, CaptureOutputArgs, CaptureOutputOutput, CaptureOutputTool,
    CreateSessionArgs, CreateSessionOutput, CreateSessionTool, KillSessionArgs, KillSessionOutput,
    KillSessionTool, ListSessionsArgs, ListSessionsOutput, ListSessionsTool, RestartSessionArgs,
    RestartSessionOutput, RestartSessionTool, SendInputArgs, SendInputOutput, SendInputTool,
    SessionInfo, ShellBackend, ShellError, ShellOutput, SwitchSessionArgs, SwitchSessionOutput,
    SwitchSessionTool, shell_tool_set,
};
