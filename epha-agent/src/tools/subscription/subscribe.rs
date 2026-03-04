//! Subscribe tool - subscribe to a Publisher
//!
//! Note: This tool requires a Publisher registry to be functional.
//! For now, it's designed to work with a pre-registered set of Publishers.

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::subscriber::{SubscriptionManager, SubscriberError};

/// Arguments for the Subscribe tool
#[derive(Deserialize, Debug)]
pub struct SubscribeArgs {
    /// The ID of the publisher to subscribe to
    pub publisher_id: String,
}

/// Output from the Subscribe tool
#[derive(Debug, Serialize)]
pub struct SubscribeOutput {
    /// Whether the subscription was successful
    pub success: bool,
    /// Publisher ID that was subscribed
    pub publisher_id: String,
    /// Message describing the result
    pub message: String,
}

/// Tool for subscribing to Publishers
pub struct SubscribeTool {
    manager: Arc<Mutex<SubscriptionManager>>,
}

impl SubscribeTool {
    /// Create a new SubscribeTool
    pub fn new(manager: Arc<Mutex<SubscriptionManager>>) -> Self {
        Self { manager }
    }
}

impl Tool for SubscribeTool {
    const NAME: &'static str = "subscribe";

    type Error = SubscriberError;
    type Args = SubscribeArgs;
    type Output = SubscribeOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "subscribe",
            "description": "Subscribe to a Publisher to receive messages from it. \
                Publishers are external data sources like Timer, Telegram, etc. \
                Once subscribed, messages from the Publisher will appear in your context.",
            "parameters": {
                "type": "object",
                "properties": {
                    "publisher_id": {
                        "type": "string",
                        "description": "The ID of the publisher to subscribe to (e.g., 'timer-service', 'telegram-bot')"
                    }
                },
                "required": ["publisher_id"]
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let publisher_id = args.publisher_id;
        let manager = self.manager.lock().await;

        // Check if already subscribed
        if manager.is_subscribed(&publisher_id) {
            return Ok(SubscribeOutput {
                success: false,
                publisher_id: publisher_id.clone(),
                message: format!("Already subscribed to publisher '{}'", publisher_id),
            });
        }

        // Note: In a full implementation, this would look up the Publisher
        // from a registry and call manager.subscribe(publisher).await.
        // For now, we return a message indicating the limitation.

        Ok(SubscribeOutput {
            success: false,
            publisher_id,
            message: "Publisher is not available for subscription. Publishers must be pre-configured in the system.".to_string(),
        })
    }
}

impl std::fmt::Display for SubscribeOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.success {
            write!(f, "Successfully subscribed to '{}'", self.publisher_id)
        } else {
            write!(f, "Failed to subscribe: {}", self.message)
        }
    }
}
