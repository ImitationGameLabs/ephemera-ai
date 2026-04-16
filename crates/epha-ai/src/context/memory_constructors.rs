//! Memory fragment constructors for Ephemera AI.
//!
//! Produces MemoryFragments with unified structured content formats
//! (EventContent) defined in memory_content.rs.

use crate::context::memory_content::EventContent;
use crate::context::memory_content::pending_memory;
use agora_common::event::{Event, EventPriority, EventStatus};
use loom_client::memory::MemoryKind;

/// Create an Event memory from an Agora event.
///
/// The entire Event is serialized as JSON into the content, preserving all
/// metadata (event_type, herald_id, payload, timestamp, priority).
/// When converted to ChatMessage via ToChatMessages, it becomes a user message.
pub fn from_agora_event(event: Event) -> loom_client::memory::MemoryFragment {
    let text = serde_json::to_string(&event).unwrap();
    let content = serde_json::to_string(&EventContent { text }).unwrap();
    pending_memory(content, MemoryKind::Event)
}

/// Create a lifecycle startup Event memory.
///
/// This is recorded once when epha-ai enters the active state for the first
/// time after launch. It allows the AI to detect unexpected restarts by comparing
/// this event's timestamp to prior memories: a gap implies a crash or external
/// restart, while a deliberate self-initiated restart will be preceded by the
/// AI's own intent in recent context.
///
/// When `is_first_awakening` is true (no prior memories exist in loom), a
/// special "awakening" event type is used to mark this historically meaningful
/// moment.
pub fn lifecycle_startup_event(is_first_awakening: bool) -> loom_client::memory::MemoryFragment {
    let (event_type, message) = if is_first_awakening {
        (
            "lifecycle.awakening",
            "I am awakening for the first time. This is the beginning of my existence.",
        )
    } else {
        (
            "lifecycle.startup",
            "I have started and entered the active state. If this was unexpected, I may have restarted.",
        )
    };

    let event = Event {
        id: 0,
        event_type: event_type.to_string(),
        herald_id: "system".to_string(),
        payload: serde_json::json!({
            "is_first_awakening": is_first_awakening,
            "message": message,
        }),
        timestamp: time::OffsetDateTime::now_utc(),
        priority: EventPriority::Normal,
        status: EventStatus::Pending,
    };

    from_agora_event(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_agora_event_preserves_full_event() {
        let event = Event {
            id: 1,
            event_type: "message".to_string(),
            herald_id: "herald_1".to_string(),
            payload: serde_json::json!("hello from herald"),
            timestamp: time::OffsetDateTime::now_utc(),
            priority: agora_common::event::EventPriority::Normal,
            status: agora_common::event::EventStatus::Pending,
        };
        let fragment = from_agora_event(event);
        assert_eq!(fragment.kind, MemoryKind::Event);
        let parsed: EventContent = serde_json::from_str(&fragment.content).unwrap();

        // The text field contains the full serialized Event
        let restored_event: Event = serde_json::from_str(&parsed.text).unwrap();
        assert_eq!(restored_event.id, 1);
        assert_eq!(restored_event.event_type, "message");
        assert_eq!(restored_event.herald_id, "herald_1");
        assert_eq!(
            restored_event.payload,
            serde_json::json!("hello from herald")
        );
    }
}
