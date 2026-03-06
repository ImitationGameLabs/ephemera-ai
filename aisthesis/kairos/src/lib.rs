//! Kairos - Time management service for Ephemera AI.
//!
//! Kairos is the "brain of time" for AI agents, allowing them to schedule,
//! query, and manage timed events. It serves as a standalone scheduling service
//! that can push events to Agora via the kairos-herald bridge.

pub mod config;
pub mod schedule;
pub mod scheduler;
pub mod server;
pub mod store;

pub use server::AppState;
