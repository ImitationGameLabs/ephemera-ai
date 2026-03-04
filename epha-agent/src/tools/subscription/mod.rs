//! Subscription management tools
//!
//! This module provides tools for managing subscriptions to Publishers.
//!
//! # Available Tools
//!
//! - [`SubscribeTool`] - Subscribe to a Publisher
//! - [`UnsubscribeTool`] - Unsubscribe from a Publisher
//! - [`ListSubscriptionsTool`] - List all subscriptions
//! - [`GetSubscriptionInfoTool`] - Get detailed info about a subscription
//!
//! # Example
//!
//! ```rust,ignore
//! use epha_agent::tools::subscription::subscription_tool_set;
//! use epha_agent::subscriber::SubscriptionManager;
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//!
//! let manager = Arc::new(Mutex::new(SubscriptionManager::with_defaults()));
//! let tools = subscription_tool_set(manager);
//! ```

mod info;
mod list;
mod subscribe;
mod unsubscribe;

pub use info::{GetSubscriptionInfoArgs, GetSubscriptionInfoOutput, GetSubscriptionInfoTool};
pub use list::{ListSubscriptionsArgs, ListSubscriptionsOutput, ListSubscriptionsTool};
pub use subscribe::{SubscribeArgs, SubscribeOutput, SubscribeTool};
pub use unsubscribe::{UnsubscribeArgs, UnsubscribeOutput, UnsubscribeTool};

use rig::tool::ToolDyn;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::subscriber::SubscriptionManager;

/// Create a set of all subscription management tools
///
/// This function creates all subscription tools configured with the same manager,
/// suitable for use with rig's agent system.
///
/// # Arguments
/// * `manager` - Shared SubscriptionManager instance wrapped in Arc<Mutex>
///
/// # Returns
/// A vector of boxed tools implementing ToolDyn
pub fn subscription_tool_set(
    manager: Arc<Mutex<SubscriptionManager>>,
) -> Vec<Box<dyn ToolDyn>> {
    vec![
        Box::new(SubscribeTool::new(manager.clone())),
        Box::new(UnsubscribeTool::new(manager.clone())),
        Box::new(ListSubscriptionsTool::new(manager.clone())),
        Box::new(GetSubscriptionInfoTool::new(manager)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_set_count() {
        let manager = Arc::new(Mutex::new(SubscriptionManager::with_defaults()));
        let tools = subscription_tool_set(manager);
        assert_eq!(tools.len(), 4);
    }
}
