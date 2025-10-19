pub mod client;
pub mod auth;
pub mod commands;
pub mod interface;

pub use client::DialogueClient;
pub use auth::{AuthManager, AuthSession};
pub use commands::{CommandHandler, CommandContext, CommandError};
pub use interface::CliInterface;