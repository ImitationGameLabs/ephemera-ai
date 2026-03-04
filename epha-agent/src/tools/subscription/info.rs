//! GetSubscriptionInfo tool - get detailed info about a specific subscription

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::subscriber::{SubscriptionManager, SubscriberError, SubscriptionInfo};

/// Arguments for the GetSubscriptionInfo tool
#[derive(Deserialize, Debug)]
pub struct GetSubscriptionInfoArgs {
    /// The ID of the publisher to get info for
    pub publisher_id: String,
}

/// Output from the GetSubscriptionInfo tool
#[derive(Debug, Serialize)]
pub struct GetSubscriptionInfoOutput {
    /// Subscription info if found
    pub subscription: Option<SubscriptionInfo>,
    /// Whether the subscription was found
    pub found: bool,
}

/// Tool for getting detailed info about a subscription
pub struct GetSubscriptionInfoTool {
    manager: Arc<Mutex<SubscriptionManager>>,
}

impl GetSubscriptionInfoTool {
    /// Create a new GetSubscriptionInfoTool
    pub fn new(manager: Arc<Mutex<SubscriptionManager>>) -> Self {
        Self { manager }
    }
}

impl Tool for GetSubscriptionInfoTool {
    const NAME: &'static str = "get_subscription_info";

    type Error = SubscriberError;
    type Args = GetSubscriptionInfoArgs;
    type Output = GetSubscriptionInfoOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "get_subscription_info",
            "description": "Get detailed information about a specific subscription, \
                including status, message statistics, and last heartbeat time.",
            "parameters": {
                "type": "object",
                "properties": {
                    "publisher_id": {
                        "type": "string",
                        "description": "The ID of the publisher to get info for"
                    }
                },
                "required": ["publisher_id"]
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let manager = self.manager.lock().await;
        let subscription = manager.get_subscription_info(&args.publisher_id);

        Ok(GetSubscriptionInfoOutput {
            found: subscription.is_some(),
            subscription,
        })
    }
}

impl std::fmt::Display for GetSubscriptionInfoOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref sub) = self.subscription {
            writeln!(f, "Subscription: {}", sub.publisher_id)?;
            writeln!(f, "  Description: {}", sub.description)?;
            writeln!(f, "  Status: {}", sub.status)?;
            writeln!(f, "  Subscribed at: {}", sub.subscribed_at)?;
            writeln!(f, "  Last heartbeat: {}", sub.last_heartbeat)?;
            writeln!(f, "  Messages received: {}", sub.messages_received)?;
            writeln!(f, "  Pending messages: {}", sub.pending_messages)?;
            Ok(())
        } else {
            write!(f, "Subscription not found")
        }
    }
}
