//! Trait definition for AgoraClient
//!
//! This trait allows for mocking in tests and dependency injection.

use async_trait::async_trait;
use agora::event::Event;
use agora::herald::HeraldInfo;

use crate::AgoraClientError;

/// Trait for Agora client operations
#[async_trait]
pub trait AgoraClientTrait: Send + Sync {
    /// Health check - verifies the service is running
    async fn health_check(&self) -> Result<String, AgoraClientError>;

    /// Fetches events for delivery (POST /events/fetch)
    /// This changes state: Pending → Delivered
    async fn fetch_events(&self, limit: Option<u32>) -> Result<Vec<Event>, AgoraClientError>;

    /// Acknowledges a single event
    async fn ack_event(&self, event_id: u64) -> Result<Event, AgoraClientError>;

    /// Batch acknowledges multiple events
    async fn ack_events(&self, event_ids: Vec<u64>) -> Result<usize, AgoraClientError>;

    /// Lists all heralds
    async fn list_heralds(&self) -> Result<Vec<HeraldInfo>, AgoraClientError>;

    /// Gets a specific herald by ID
    async fn get_herald(&self, id: &str) -> Result<HeraldInfo, AgoraClientError>;

    /// Gets the base URL this client is configured to use
    fn base_url(&self) -> &str;
}
