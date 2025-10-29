//! Context module for Ephemera AI
//!
//! Provides context management and memory fragment serialization
//! for creating and managing AI context state.

use epha_agent::context::ContextSerialize;
use loom_client::memory::MemoryFragment;

// Re-export all public types
pub use ephemera_context::{EphemeraContext};
pub use memory_fragment_list::MemoryFragmentList;

mod ephemera_context;
mod memory_fragment_list;