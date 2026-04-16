//! Context module for Ephemera AI
//!
//! Provides context management and memory fragment serialization
//! for creating and managing AI context state.

#![allow(unused)]

mod ephemera_context;
mod error;
mod memory_constructors;
mod memory_content;
mod memory_fragment_list;
mod memory_observability;

// Re-export public types
pub use ephemera_context::*;
pub use error::PinError;
pub use error::PinnedTokenBudgetError;
pub use error::TokenBudgetError;
pub use memory_constructors::{from_agora_event, lifecycle_startup_event};
pub use memory_content::*;
pub use memory_fragment_list::*;
pub(crate) use memory_observability::{fragment_log_meta, summarize_batch_log_meta};
