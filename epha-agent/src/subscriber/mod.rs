//! Subscriber module - manages subscriptions to Publishers
//!
//! This module provides the SubscriptionManager which handles:
//! - Subscribing to and unsubscribing from Publishers
//! - Caching pending messages for epha-ai
//! - Heartbeat monitoring and status tracking
//!
//! # Architecture
//!
//! ```text
//! Publisher → subscribe() → SubscriptionManager
//!                                │
//!                                ├── subscriptions: HashMap<String, ActiveSubscription>
//!                                │
//!                                └── pending_messages: VecDeque<PublisherMessage>
//!                                        │
//!                                        └── drain_pending_messages() → Vec<PublisherMessage>
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use epha_agent::subscriber::SubscriptionManager;
//! use epha_agent::publisher::Publisher;
//!
//! let manager = SubscriptionManager::with_defaults();
//!
//! // Subscribe to a publisher
//! manager.subscribe(Box::new(my_publisher)).await?;
//!
//! // In cognitive cycle, drain pending messages
//! let messages = manager.drain_pending_messages();
//!
//! // List all subscriptions
//! let subs = manager.list_subscriptions();
//! ```

mod error;
mod manager;
mod types;

pub use error::SubscriberError;
pub use manager::SubscriptionManager;
pub use types::{SubscriberConfig, SubscriptionInfo, SubscriptionStatus};
