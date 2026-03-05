//! Event types for Agora event hub.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Unique identifier for an event.
pub type EventId = u64;

/// Event priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EventPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

/// Event processing status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    #[default]
    Pending,
    Delivered,
    Acked,
}

/// An event produced by a herald.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier.
    pub id: EventId,
    /// Event type (e.g., "timer.trigger", "chat.message").
    pub event_type: String,
    /// ID of the herald that produced this event.
    pub herald_id: String,
    /// Event payload as JSON.
    pub payload: serde_json::Value,
    /// Event priority.
    pub priority: EventPriority,
    /// Event creation timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    /// Event processing status.
    pub status: EventStatus,
}

/// Request to create a new event.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateEventRequest {
    /// Event type (e.g., "timer.trigger", "chat.message").
    pub event_type: String,
    /// ID of the herald that produced this event.
    pub herald_id: String,
    /// Event priority (defaults to Normal).
    #[serde(default)]
    pub priority: EventPriority,
    /// Event payload as JSON.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// Request to update event status.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateEventRequest {
    /// New event status.
    pub status: EventStatus,
}

/// Request to batch update event status.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchUpdateEventsRequest {
    /// Event IDs to update.
    pub event_ids: Vec<EventId>,
    /// New status for all events.
    pub status: EventStatus,
}

/// Response for batch update operation.
#[derive(Debug, Clone, Serialize)]
pub struct BatchUpdateEventsResponse {
    /// Number of events updated.
    pub updated: usize,
}

/// Events list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsListResponse {
    /// List of events.
    pub events: Vec<Event>,
    /// Total count.
    pub total: usize,
}
