//! Agora client library for Ephemera AI.
//!
//! This client provides a convenient interface for interacting with
//! the Agora event hub.

mod client;

pub use client::{AgoraClient, AgoraClientError};

// Re-export commonly used types from agora
pub use agora::event::{Event, EventId, EventPriority, EventStatus};
pub use agora::herald::{HeraldInfo, HeraldStatus};
