//! Error types for Publisher operations

use thiserror::Error;

/// Errors that can occur during Publisher operations
#[derive(Debug, Error)]
pub enum PublisherError {
    #[error("Connection failed for publisher '{publisher_id}': {message}")]
    ConnectionFailed {
        publisher_id: String,
        message: String,
    },

    #[error("Authentication failed for publisher '{publisher_id}': {message}")]
    AuthFailed {
        publisher_id: String,
        message: String,
    },

    #[error("Publisher '{publisher_id}' is not subscribed")]
    NotSubscribed { publisher_id: String },

    #[error("Publisher '{publisher_id}' is already subscribed")]
    AlreadySubscribed { publisher_id: String },

    #[error("Health check failed for publisher '{publisher_id}': {message}")]
    HealthCheckFailed {
        publisher_id: String,
        message: String,
    },

    #[error("Failed to send message: {message}")]
    SendFailed { message: String },

    #[error("Rate limited for publisher '{publisher_id}'")]
    RateLimited { publisher_id: String },

    #[error("Internal error in publisher '{publisher_id}': {message}")]
    Internal {
        publisher_id: String,
        message: String,
    },

    #[error("Invalid message format: {details}")]
    InvalidMessage { details: String },

    #[error("Channel closed for publisher '{publisher_id}'")]
    ChannelClosed { publisher_id: String },
}

impl PublisherError {
    pub fn connection_failed(publisher_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ConnectionFailed {
            publisher_id: publisher_id.into(),
            message: message.into(),
        }
    }

    pub fn auth_failed(publisher_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::AuthFailed {
            publisher_id: publisher_id.into(),
            message: message.into(),
        }
    }

    pub fn not_subscribed(publisher_id: impl Into<String>) -> Self {
        Self::NotSubscribed {
            publisher_id: publisher_id.into(),
        }
    }

    pub fn already_subscribed(publisher_id: impl Into<String>) -> Self {
        Self::AlreadySubscribed {
            publisher_id: publisher_id.into(),
        }
    }

    pub fn health_check_failed(publisher_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::HealthCheckFailed {
            publisher_id: publisher_id.into(),
            message: message.into(),
        }
    }

    pub fn internal(publisher_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Internal {
            publisher_id: publisher_id.into(),
            message: message.into(),
        }
    }
}
