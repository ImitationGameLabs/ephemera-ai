//! SubscriptionManager - manages subscriptions to Publishers
//!
//! This module implements the subscriber side of the Publisher-Subscriber pattern.
//! It manages active subscriptions, caches pending messages, and handles heartbeat detection.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::publisher::{Publisher, PublisherMessage};
use super::error::SubscriberError;
use super::types::{SubscriptionInfo, SubscriptionStatus, SubscriberConfig};

/// Active subscription data
struct ActiveSubscription {
    /// Publisher description
    description: String,
    /// When the subscription was created
    subscribed_at: time::OffsetDateTime,
    /// Last successful heartbeat
    last_heartbeat: time::OffsetDateTime,
    /// Current status
    status: SubscriptionStatus,
    /// Total messages received
    messages_received: u64,
    /// Message receiver from the Publisher
    receiver: mpsc::Receiver<PublisherMessage>,
}

/// SubscriptionManager manages subscriptions to multiple Publishers
///
/// It provides:
/// - Subscription management (subscribe/unsubscribe)
/// - Message caching for retrieval by epha-ai
/// - Heartbeat monitoring and status tracking
pub struct SubscriptionManager {
    /// Active subscriptions indexed by publisher_id
    subscriptions: Mutex<HashMap<String, ActiveSubscription>>,

    /// Pending messages waiting to be processed by epha-ai
    pending_messages: Mutex<VecDeque<PublisherMessage>>,

    /// Configuration for heartbeat behavior
    config: SubscriberConfig,
}

impl SubscriptionManager {
    /// Create a new SubscriptionManager with the given configuration
    pub fn new(config: SubscriberConfig) -> Self {
        Self {
            subscriptions: Mutex::new(HashMap::new()),
            pending_messages: Mutex::new(VecDeque::new()),
            config,
        }
    }

    /// Create a new SubscriptionManager with default configuration
    pub fn with_defaults() -> Self {
        Self::new(SubscriberConfig::default())
    }

    /// Subscribe to a Publisher
    ///
    /// This method:
    /// 1. Calls publisher.subscribe() to get a message receiver
    /// 2. Stores the subscription info
    /// 3. Starts receiving messages in a background task
    pub async fn subscribe(&self, publisher: Box<dyn Publisher>) -> Result<(), SubscriberError> {
        let publisher_id = publisher.id().to_string();
        let description = publisher.description().to_string();

        // Check if already subscribed
        {
            let subscriptions = self.subscriptions.lock().unwrap();
            if subscriptions.contains_key(&publisher_id) {
                return Err(SubscriberError::already_subscribed(&publisher_id));
            }
        }

        // Subscribe to the publisher
        let receiver = publisher.subscribe().await
            .map_err(|e| SubscriberError::subscribe_failed(&publisher_id, e.to_string()))?;

        // Store the subscription
        let now = time::OffsetDateTime::now_utc();
        let subscription = ActiveSubscription {
            description,
            subscribed_at: now,
            last_heartbeat: now,
            status: SubscriptionStatus::Active,
            messages_received: 0,
            receiver,
        };

        {
            let mut subscriptions = self.subscriptions.lock().unwrap();
            subscriptions.insert(publisher_id.clone(), subscription);
        }

        info!("Subscribed to publisher: {}", publisher_id);
        Ok(())
    }

    /// Unsubscribe from a Publisher
    ///
    /// This stops receiving messages and removes the subscription.
    pub async fn unsubscribe(&self, publisher_id: &str) -> Result<(), SubscriberError> {
        let mut subscriptions = self.subscriptions.lock().unwrap();

        if let Some(subscription) = subscriptions.remove(publisher_id) {
            // Note: We can't call publisher.unsubscribe() here since we don't have
            // access to the Publisher object. The receiver will be dropped,
            // which should signal the Publisher to stop.
            drop(subscription.receiver);
            info!("Unsubscribed from publisher: {}", publisher_id);
            Ok(())
        } else {
            Err(SubscriberError::not_subscribed(publisher_id))
        }
    }

    /// Drain all pending messages
    ///
    /// Called by epha-ai before each cognitive_cycle to get new messages.
    /// Also collects any messages available from active subscriptions.
    pub fn drain_pending_messages(&self) -> Vec<PublisherMessage> {
        // First, collect from all active subscriptions
        {
            let mut subscriptions = self.subscriptions.lock().unwrap();
            for (_publisher_id, subscription) in subscriptions.iter_mut() {
                // Non-blocking receive from the subscription's receiver
                while let Ok(msg) = subscription.receiver.try_recv() {
                    subscription.messages_received += 1;
                    self.pending_messages.lock().unwrap().push_back(msg);
                }
            }
        }

        // Then drain the pending queue
        let mut pending = self.pending_messages.lock().unwrap();
        pending.drain(..).collect()
    }

    /// List all subscriptions with their current status
    pub fn list_subscriptions(&self) -> Vec<SubscriptionInfo> {
        let subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.iter().map(|(id, sub)| {
            SubscriptionInfo {
                publisher_id: id.clone(),
                description: sub.description.clone(),
                status: sub.status,
                subscribed_at: sub.subscribed_at,
                last_heartbeat: sub.last_heartbeat,
                messages_received: sub.messages_received,
                pending_messages: self.pending_messages.lock().unwrap().len(),
            }
        }).collect()
    }

    /// Get detailed info about a specific subscription
    pub fn get_subscription_info(&self, publisher_id: &str) -> Option<SubscriptionInfo> {
        let subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.get(publisher_id).map(|sub| {
            SubscriptionInfo {
                publisher_id: publisher_id.to_string(),
                description: sub.description.clone(),
                status: sub.status,
                subscribed_at: sub.subscribed_at,
                last_heartbeat: sub.last_heartbeat,
                messages_received: sub.messages_received,
                pending_messages: self.pending_messages.lock().unwrap().len(),
            }
        })
    }

    /// Check if a publisher is subscribed
    pub fn is_subscribed(&self, publisher_id: &str) -> bool {
        let subscriptions = self.subscriptions.lock().unwrap();
        subscriptions.contains_key(publisher_id)
    }

    /// Get the number of active subscriptions
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.lock().unwrap().len()
    }

    /// Run a single heartbeat check cycle
    ///
    /// This method checks all subscriptions and updates their status.
    /// Returns a list of publishers that became degraded or disconnected.
    pub async fn run_heartbeat_cycle(&self) -> Vec<(String, SubscriptionStatus)> {
        let now = time::OffsetDateTime::now_utc();
        let degraded_threshold = Duration::from_secs(self.config.degraded_timeout_seconds);
        let disconnect_threshold = Duration::from_secs(self.config.disconnect_timeout_seconds);

        let mut status_changes = Vec::new();
        let mut to_disconnect = Vec::new();

        {
            let mut subscriptions = self.subscriptions.lock().unwrap();
            for (publisher_id, subscription) in subscriptions.iter_mut() {
                let elapsed = now - subscription.last_heartbeat;
                let elapsed_duration = Duration::try_from(elapsed).unwrap_or_default();

                let new_status = if elapsed_duration > disconnect_threshold {
                    SubscriptionStatus::Disconnected
                } else if elapsed_duration > degraded_threshold {
                    SubscriptionStatus::Degraded
                } else {
                    SubscriptionStatus::Active
                };

                if new_status != subscription.status {
                    subscription.status = new_status;
                    status_changes.push((publisher_id.clone(), new_status));

                    if new_status == SubscriptionStatus::Disconnected {
                        to_disconnect.push(publisher_id.clone());
                    }
                }
            }
        }

        // Auto-disconnect publishers that are disconnected
        for publisher_id in to_disconnect {
            warn!("Auto-disconnecting publisher due to heartbeat timeout: {}", publisher_id);
            if let Err(e) = self.unsubscribe(&publisher_id).await {
                error!("Failed to auto-disconnect publisher {}: {}", publisher_id, e);
            }
        }

        status_changes
    }

    /// Get the configuration
    pub fn config(&self) -> &SubscriberConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = SubscriptionManager::with_defaults();
        assert_eq!(manager.subscription_count(), 0);
    }

    #[test]
    fn test_manager_with_custom_config() {
        let config = SubscriberConfig {
            heartbeat_interval_seconds: 10,
            degraded_timeout_seconds: 20,
            disconnect_timeout_seconds: 30,
        };
        let manager = SubscriptionManager::new(config);
        assert_eq!(manager.config().heartbeat_interval_seconds, 10);
    }

    #[test]
    fn test_list_empty_subscriptions() {
        let manager = SubscriptionManager::with_defaults();
        let subs = manager.list_subscriptions();
        assert!(subs.is_empty());
    }

    #[test]
    fn test_drain_empty_messages() {
        let manager = SubscriptionManager::with_defaults();
        let messages = manager.drain_pending_messages();
        assert!(messages.is_empty());
    }
}
