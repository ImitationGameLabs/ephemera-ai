//! Unsubscribe tool - unsubscribe from a Publisher

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::subscriber::{SubscriptionManager, SubscriberError};

/// Arguments for the Unsubscribe tool
#[derive(Deserialize, Debug)]
pub struct UnsubscribeArgs {
    /// The ID of the publisher to unsubscribe from
    pub publisher_id: String,
}

/// Output from the Unsubscribe tool
#[derive(Debug, Serialize)]
pub struct UnsubscribeOutput {
    /// Whether the unsubscription was successful
    pub success: bool,
    /// Publisher ID that was unsubscribed
    pub publisher_id: String,
    /// Message describing the result
    pub message: String,
}

/// Tool for unsubscribing from Publishers
pub struct UnsubscribeTool {
    manager: Arc<Mutex<SubscriptionManager>>,
}

impl UnsubscribeTool {
    /// Create a new UnsubscribeTool
    pub fn new(manager: Arc<Mutex<SubscriptionManager>>) -> Self {
        Self { manager }
    }
}

impl Tool for UnsubscribeTool {
    const NAME: &'static str = "unsubscribe";

    type Error = SubscriberError;
    type Args = UnsubscribeArgs;
    type Output = UnsubscribeOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "unsubscribe",
            "description": "Unsubscribe from a Publisher. You will stop receiving messages from this Publisher.",
            "parameters": {
                "type": "object",
                "properties": {
                    "publisher_id": {
                        "type": "string",
                        "description": "The ID of the publisher to unsubscribe from"
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

        if !manager.is_subscribed(&publisher_id) {
            return Ok(UnsubscribeOutput {
                success: false,
                publisher_id: publisher_id.clone(),
                message: format!("Not subscribed to publisher '{}'", publisher_id),
            });
        }

        drop(manager); // Release lock before async operation

        let manager = self.manager.lock().await;
        manager.unsubscribe(&publisher_id).await?;

        Ok(UnsubscribeOutput {
            success: true,
            publisher_id,
            message: "Successfully unsubscribed".to_string(),
        })
    }
}

impl std::fmt::Display for UnsubscribeOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.success {
            write!(f, "Successfully unsubscribed from '{}'", self.publisher_id)
        } else {
            write!(f, "Failed to unsubscribe: {}", self.message)
        }
    }
}
