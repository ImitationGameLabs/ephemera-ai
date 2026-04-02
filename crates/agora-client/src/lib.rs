//! Agora client library for Ephemera AI.
//!
//! This client provides a convenient interface for interacting with
//! the Agora event hub.

mod client;
pub mod mock;
mod trait_def;

pub use client::{AgoraClient, AgoraClientError};
pub use trait_def::AgoraClientTrait;

// Re-export commonly used types from agora
pub use agora_common::event::{Event, EventId, EventPriority, EventStatus};
pub use agora_common::herald::{HeraldInfo, HeraldStatus};
