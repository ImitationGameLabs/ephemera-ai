//! Publisher trait definition
//!
//! All Publishers must implement this trait to be subscribable by SubscriptionManager.

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::error::PublisherError;
use super::message::{HealthStatus, PublisherMessage};

/// Publisher trait - all subscribable data sources must implement this interface
///
/// Publisher is an abstraction for external data sources, responsible for:
/// - Generating compliant PublisherMessages
/// - Managing its own connection and health status
/// - Pushing messages to subscribers via channel
#[async_trait]
pub trait Publisher: Send + Sync {
    /// Unique identifier of the Publisher
    ///
    /// This ID is used to identify the subscription in SubscriptionManager
    fn id(&self) -> &str;

    /// Description of the Publisher
    ///
    /// Used for display in subscription lists and logs
    fn description(&self) -> &str;

    /// Subscribe to the Publisher, returning a message receiver
    ///
    /// After calling this method, the Publisher starts sending messages to the returned receiver.
    /// Messages are delivered via tokio mpsc channel.
    ///
    /// # Returns
    /// - `Ok(receiver)`: Successfully subscribed, returns message receiver
    /// - `Err(PublisherError)`: Subscription failed
    async fn subscribe(&self) -> Result<mpsc::Receiver<PublisherMessage>, PublisherError>;

    /// Unsubscribe from the Publisher
    ///
    /// Stops sending messages and cleans up resources.
    /// This method should be idempotent - calling it again on an already unsubscribed Publisher should not error.
    async fn unsubscribe(&self) -> Result<(), PublisherError>;

    /// Health check
    ///
    /// Returns the current health status of the Publisher.
    /// SubscriptionManager uses this method for heartbeat detection.
    async fn health_check(&self) -> Result<HealthStatus, PublisherError>;
}

#[cfg(test)]
mod tests {
    // Trait definition tests would be in implementation-specific test files
    // This module is intentionally minimal
}
