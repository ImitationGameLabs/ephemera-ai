//! Error types for Subscriber operations

use thiserror::Error;

/// Errors that can occur during Subscriber operations
#[derive(Debug, Error)]
pub enum SubscriberError {
    #[error("Publisher '{0}' is not subscribed")]
    NotSubscribed(String),

    #[error("Publisher '{0}' is already subscribed")]
    AlreadySubscribed(String),

    #[error("Failed to subscribe to publisher '{publisher_id}': {message}")]
    SubscribeFailed {
        publisher_id: String,
        message: String,
    },

    #[error("Failed to unsubscribe from publisher '{publisher_id}': {message}")]
    UnsubscribeFailed {
        publisher_id: String,
        message: String,
    },

    #[error("Health check failed for publisher '{publisher_id}': {message}")]
    HealthCheckFailed {
        publisher_id: String,
        message: String,
    },

    #[error("Publisher '{publisher_id}' connection lost: {message}")]
    ConnectionLost {
        publisher_id: String,
        message: String,
    },

    #[error("Message queue is full for publisher '{0}'")]
    QueueFull(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl SubscriberError {
    pub fn not_subscribed(publisher_id: impl Into<String>) -> Self {
        Self::NotSubscribed(publisher_id.into())
    }

    pub fn already_subscribed(publisher_id: impl Into<String>) -> Self {
        Self::AlreadySubscribed(publisher_id.into())
    }

    pub fn subscribe_failed(publisher_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::SubscribeFailed {
            publisher_id: publisher_id.into(),
            message: message.into(),
        }
    }

    pub fn unsubscribe_failed(publisher_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::UnsubscribeFailed {
            publisher_id: publisher_id.into(),
            message: message.into(),
        }
    }

    pub fn connection_lost(publisher_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ConnectionLost {
            publisher_id: publisher_id.into(),
            message: message.into(),
        }
    }
}
