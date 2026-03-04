//! Types for Subscription management

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Subscription status indicating the health of the connection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    /// Normal operation - heartbeat successful
    Active,
    /// Heartbeat timeout but not yet disconnected
    Degraded,
    /// Connection lost, auto-unsubscribe pending
    Disconnected,
}

impl std::fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Degraded => write!(f, "degraded"),
            Self::Disconnected => write!(f, "disconnected"),
        }
    }
}

/// Information about a subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    /// Publisher ID
    pub publisher_id: String,

    /// Publisher description
    pub description: String,

    /// Current status
    pub status: SubscriptionStatus,

    /// When the subscription was created
    #[serde(with = "time::serde::iso8601")]
    pub subscribed_at: OffsetDateTime,

    /// Last successful heartbeat
    #[serde(with = "time::serde::iso8601")]
    pub last_heartbeat: OffsetDateTime,

    /// Total messages received
    pub messages_received: u64,

    /// Messages waiting in queue
    pub pending_messages: usize,
}

/// Configuration for SubscriptionManager heartbeat behavior
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriberConfig {
    /// Interval between heartbeat checks in seconds
    pub heartbeat_interval_seconds: u64,

    /// Time without heartbeat before status becomes Degraded
    pub degraded_timeout_seconds: u64,

    /// Time without heartbeat before auto-disconnect
    pub disconnect_timeout_seconds: u64,
}

impl Default for SubscriberConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_seconds: 30,
            degraded_timeout_seconds: 60,
            disconnect_timeout_seconds: 120,
        }
    }
}
