//! Session management tools
//!
//! This module provides tools for managing shell sessions:
//! - List sessions
//! - Create new sessions
//! - Switch between sessions
//! - Kill sessions
//! - Restart sessions

mod create;
mod kill;
mod list;
mod restart;
mod switch;

pub use create::{CreateSessionArgs, CreateSessionOutput, CreateSessionTool};
pub use kill::{KillSessionArgs, KillSessionOutput, KillSessionTool};
pub use list::{ListSessionsArgs, ListSessionsOutput, ListSessionsTool};
pub use restart::{RestartSessionArgs, RestartSessionOutput, RestartSessionTool};
pub use switch::{SwitchSessionArgs, SwitchSessionOutput, SwitchSessionTool};
