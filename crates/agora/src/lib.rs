//! Agora - Event hub for Ephemera AI.
//!
//! Agora serves as the central event hub where heralds (event producers)
//! push events and consumers (like epha-ai) pull and acknowledge them.

pub mod config;
pub mod event;
pub mod handlers;
pub mod herald;
pub mod queue;
pub mod server;

pub use server::AppState;
