mod raw_client;
mod authenticated_client;

// Re-export main types for backward compatibility
pub use raw_client::*;
pub use authenticated_client::*;