//! Event queue implementations.

mod sqlite;
mod state;

pub use sqlite::SqliteEventStore;
pub use state::DeliveryCache;

use crate::config::RetryConfig;
use crate::event::{CreateEventRequest, Event, EventId, EventStatus};
use anyhow::Result;

/// Combined event queue with persistence and delivery tracking.
pub struct EventQueue {
    store: SqliteEventStore,
    cache: DeliveryCache,
    retry_config: RetryConfig,
}

impl EventQueue {
    pub async fn new(database_path: &str, retry_config: RetryConfig) -> Result<Self> {
        let store = SqliteEventStore::new(database_path).await?;
        let cache = DeliveryCache::new();

        // Load pending events from SQLite
        let pending = store.load_all().await?;
        cache.load_pending(pending).await;

        Ok(Self { store, cache, retry_config })
    }

    /// Create new event.
    pub async fn push(&self, req: CreateEventRequest) -> Result<Event> {
        let event = self.store.insert(req).await?;
        self.cache.add_pending(event.clone()).await;
        Ok(event)
    }

    /// Get events for delivery (pending + retries).
    pub async fn fetch(&self, limit: u32) -> Vec<Event> {
        self.cache.get_deliverable(limit, &self.retry_config).await
    }

    /// Acknowledge event. Returns error if SQLite delete fails.
    pub async fn ack(&self, id: EventId) -> Result<Option<Event>> {
        // 1. Delete from SQLite first (must succeed)
        let deleted = self.store.delete(id).await?;
        if !deleted {
            return Ok(None);
        }

        // 2. Remove from memory immediately
        Ok(self.cache.remove(id).await)
    }

    /// Batch acknowledge.
    ///
    /// **Non-atomic**: If ack #3 of 5 fails, the first 2 are still acked.
    /// Callers receive list of successfully acked event IDs, not transaction rollback.
    pub async fn batch_ack(&self, ids: Vec<EventId>) -> Result<Vec<EventId>> {
        let mut acked_ids = Vec::with_capacity(ids.len());
        for id in ids {
            if self.ack(id).await?.is_some() {
                acked_ids.push(id);
            }
        }
        Ok(acked_ids)
    }

    /// Update event status (for compatibility).
    /// Only supports Acked status, which triggers ack().
    pub async fn update_status(&self, id: EventId, status: EventStatus) -> Result<Option<Event>> {
        if status == EventStatus::Acked {
            self.ack(id).await
        } else {
            // Delivered status is handled internally by fetch()
            Ok(None)
        }
    }

    /// Batch update event status (for compatibility).
    /// Only supports Acked status.
    pub async fn batch_update_status(
        &self,
        ids: Vec<EventId>,
        status: EventStatus,
    ) -> Result<Vec<EventId>> {
        if status == EventStatus::Acked { self.batch_ack(ids).await } else { Ok(Vec::new()) }
    }
}
