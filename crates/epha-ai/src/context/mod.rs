//! Context module for Ephemera AI
//!
//! Provides context management and memory fragment serialization
//! for creating and managing AI context state.

#![allow(unused)]

mod ephemera_context;
mod memory_constructors;
mod memory_content;
mod memory_fragment_list;

// Re-export public types
pub use ephemera_context::*;
pub use memory_content::*;
pub use memory_fragment_list::*;
