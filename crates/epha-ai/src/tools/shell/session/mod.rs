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

pub use create::CreateSessionTool;
pub use kill::KillSessionTool;
pub use list::ListSessionsTool;
pub use restart::RestartSessionTool;
pub use switch::SwitchSessionTool;
