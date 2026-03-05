//! In-memory event queue implementation.

use crate::event::{Event, EventId, EventStatus, CreateEventRequest};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;
use time::OffsetDateTime;

/// In-memory event queue.
#[derive(Debug)]
pub struct MemoryEventQueue {
    events: RwLock<HashMap<EventId, Event>>,
    next_id: AtomicU64,
}

impl Default for MemoryEventQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryEventQueue {
    /// Creates a new empty queue.
    pub fn new() -> Self {
        Self {
            events: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// Pushes a new event to the queue.
    pub async fn push(&self, request: CreateEventRequest) -> Event {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let event = Event {
            id,
            event_type: request.event_type,
            herald_id: request.herald_id,
            payload: request.payload,
            priority: request.priority,
            timestamp: OffsetDateTime::now_utc(),
            status: EventStatus::Pending,
        };

        let mut events = self.events.write().await;
        events.insert(id, event.clone());
        event
    }

    /// Gets events by status.
    pub async fn get_by_status(&self, status: Option<EventStatus>, limit: Option<u32>) -> Vec<Event> {
        let events = self.events.read().await;
        let limit = limit.unwrap_or(100) as usize;

        events
            .values()
            .filter(|e| status.is_none() || e.status == status.unwrap())
            .take(limit)
            .cloned()
            .collect()
    }

    /// Gets a single event by ID.
    pub async fn get(&self, id: EventId) -> Option<Event> {
        let events = self.events.read().await;
        events.get(&id).cloned()
    }

    /// Updates event status.
    pub async fn update_status(&self, id: EventId, status: EventStatus) -> Option<Event> {
        let mut events = self.events.write().await;
        if let Some(event) = events.get_mut(&id) {
            event.status = status;
            Some(event.clone())
        } else {
            None
        }
    }

    /// Batch updates event status.
    /// Returns the number of events updated.
    pub async fn batch_update_status(&self, ids: Vec<EventId>, status: EventStatus) -> usize {
        let mut events = self.events.write().await;
        let mut updated = 0;

        for id in ids {
            if let Some(event) = events.get_mut(&id) {
                event.status = status;
                updated += 1;
            }
        }

        updated
    }

    /// Removes old acked events (cleanup).
    pub async fn cleanup_acked(&self, older_than_seconds: i64) -> usize {
        let mut events = self.events.write().await;
        let now = OffsetDateTime::now_utc();
        let initial_len = events.len();

        events.retain(|_, e| {
            e.status != EventStatus::Acked
                || (now - e.timestamp).whole_seconds() < older_than_seconds
        });

        initial_len - events.len()
    }
}
