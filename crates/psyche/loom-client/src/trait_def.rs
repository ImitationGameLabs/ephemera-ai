//! Trait definition for LoomClient
//!
//! This trait allows for mocking in tests and dependency injection.

use async_trait::async_trait;
use loom_common::models::*;
use loom_common::types::MemoryFragment;

use crate::LoomClientError;

/// Trait for Loom client operations
#[async_trait]
pub trait LoomClientTrait: Send + Sync {
    /// Health check - verify the service is running
    async fn health_check(&self) -> Result<serde_json::Value, LoomClientError>;

    /// Create a new memory fragment
    async fn create_memory(
        &self,
        request: CreateMemoryRequest,
    ) -> Result<MemoryResponse, LoomClientError>;

    /// Create a single memory fragment (backward compatibility convenience method)
    async fn create_single_memory(
        &self,
        fragment: MemoryFragment,
    ) -> Result<MemoryResponse, LoomClientError>;

    /// Get a specific memory fragment by ID
    async fn get_memory(&self, id: i64) -> Result<MemoryResponse, LoomClientError>;

    /// Delete a memory fragment by ID
    async fn delete_memory(&self, id: i64) -> Result<(), LoomClientError>;

    /// Get recent memory fragments
    async fn get_recent_memories(&self, limit: usize) -> Result<MemoryResponse, LoomClientError>;

    /// Get memory fragments within a time range (timeline view)
    async fn get_timeline_memory(
        &self,
        from: &str,
        to: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<MemoryResponse, LoomClientError>;

    /// Get all pinned memories
    async fn get_pinned_memories(&self) -> Result<PinnedMemoriesResponse, LoomClientError>;

    /// Pin a memory by ID
    async fn pin_memory(
        &self,
        memory_id: i64,
        reason: Option<String>,
    ) -> Result<PinnedMemory, LoomClientError>;

    /// Unpin a memory by ID
    async fn unpin_memory(&self, memory_id: i64) -> Result<(), LoomClientError>;

    /// Get the base URL this client is configured to use
    fn base_url(&self) -> &str;
}
