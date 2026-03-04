//! ListSubscriptions tool - list all current subscriptions

use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::subscriber::{SubscriptionManager, SubscriberError, SubscriptionInfo};

/// Arguments for the ListSubscriptions tool (empty)
#[derive(Deserialize, Debug)]
pub struct ListSubscriptionsArgs {}

/// Output from the ListSubscriptions tool
#[derive(Debug, Serialize)]
pub struct ListSubscriptionsOutput {
    /// List of subscription info
    pub subscriptions: Vec<SubscriptionInfo>,
    /// Total count
    pub count: usize,
}

/// Tool for listing all subscriptions
pub struct ListSubscriptionsTool {
    manager: Arc<Mutex<SubscriptionManager>>,
}

impl ListSubscriptionsTool {
    /// Create a new ListSubscriptionsTool
    pub fn new(manager: Arc<Mutex<SubscriptionManager>>) -> Self {
        Self { manager }
    }
}

impl Tool for ListSubscriptionsTool {
    const NAME: &'static str = "list_subscriptions";

    type Error = SubscriberError;
    type Args = ListSubscriptionsArgs;
    type Output = ListSubscriptionsOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "list_subscriptions",
            "description": "List all current subscriptions with their status. \
                Shows publisher IDs, connection status (active/degraded/disconnected), \
                and message statistics.",
            "parameters": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }))
        .expect("Tool definition should be valid JSON")
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let manager = self.manager.lock().await;
        let subscriptions = manager.list_subscriptions();
        let count = subscriptions.len();

        Ok(ListSubscriptionsOutput {
            subscriptions,
            count,
        })
    }
}

impl std::fmt::Display for ListSubscriptionsOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.subscriptions.is_empty() {
            write!(f, "No active subscriptions")
        } else {
            writeln!(f, "Active Subscriptions ({}):", self.count)?;
            for sub in &self.subscriptions {
                writeln!(
                    f,
                    "  - {} [{}] - {} messages received",
                    sub.publisher_id,
                    sub.status,
                    sub.messages_received
                )?;
            }
            Ok(())
        }
    }
}
