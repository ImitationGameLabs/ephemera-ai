//! Memory fragment constructors for Ephemera AI.
//!
//! Produces MemoryFragments with unified structured content formats
//! (EventContent) defined in memory_content.rs.

use crate::context::memory_content::EventContent;
use crate::context::memory_content::pending_memory;
use agora::event::Event;
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
            priority: agora::event::EventPriority::Normal,
            status: agora::event::EventStatus::Pending,
        };
        let fragment = from_agora_event(event);
        assert_eq!(fragment.kind, MemoryKind::Event);
        let parsed: EventContent = serde_json::from_str(&fragment.content).unwrap();

        // The text field contains the full serialized Event
        let restored_event: Event = serde_json::from_str(&parsed.text).unwrap();
        assert_eq!(restored_event.id, 1);
        assert_eq!(restored_event.event_type, "message");
        assert_eq!(restored_event.herald_id, "herald_1");
        assert_eq!(restored_event.payload, serde_json::json!("hello from herald"));
    }
}
