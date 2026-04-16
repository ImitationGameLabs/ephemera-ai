use super::memory_content::{ActionMemoryContent, EventContent, ThoughtContent};
use agora_common::event::Event;
use loom_client::memory::{MemoryFragment, MemoryKind};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryLogMeta {
    pub kind: &'static str,
    pub event_type: Option<String>,
    pub tool_call_count: Option<usize>,
    pub text_len: Option<usize>,
    pub parse_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryBatchLogMeta {
    pub total: usize,
    pub kind_counts: BTreeMap<&'static str, usize>,
    pub event_type_counts: BTreeMap<String, usize>,
    pub parse_fallback_count: usize,
}

fn kind_name(kind: &MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Thought => "thought",
        MemoryKind::Event => "event",
        MemoryKind::Action => "action",
        MemoryKind::Unknown => "unknown",
    }
}

pub fn fragment_log_meta(fragment: &MemoryFragment) -> MemoryLogMeta {
    match fragment.kind {
        MemoryKind::Thought => match serde_json::from_str::<ThoughtContent>(&fragment.content) {
            Ok(thought) => MemoryLogMeta {
                kind: kind_name(&fragment.kind),
                event_type: None,
                tool_call_count: None,
                text_len: Some(thought.text.len()),
                parse_fallback: false,
            },
            Err(_) => MemoryLogMeta {
                kind: kind_name(&fragment.kind),
                event_type: None,
                tool_call_count: None,
                text_len: Some(fragment.content.len()),
                parse_fallback: true,
            },
        },
        MemoryKind::Action => {
            match serde_json::from_str::<ActionMemoryContent>(&fragment.content) {
                Ok(action) => MemoryLogMeta {
                    kind: kind_name(&fragment.kind),
                    event_type: None,
                    tool_call_count: Some(action.tool_calls.len()),
                    text_len: None,
                    parse_fallback: false,
                },
                Err(_) => MemoryLogMeta {
                    kind: kind_name(&fragment.kind),
                    event_type: None,
                    tool_call_count: None,
                    text_len: Some(fragment.content.len()),
                    parse_fallback: true,
                },
            }
        }
        MemoryKind::Event => match serde_json::from_str::<EventContent>(&fragment.content) {
            Ok(event_content) => match serde_json::from_str::<Event>(&event_content.text) {
                Ok(event) => MemoryLogMeta {
                    kind: kind_name(&fragment.kind),
                    event_type: Some(event.event_type),
                    tool_call_count: None,
                    text_len: None,
                    parse_fallback: false,
                },
                Err(_) => MemoryLogMeta {
                    kind: kind_name(&fragment.kind),
                    event_type: None,
                    tool_call_count: None,
                    text_len: Some(event_content.text.len()),
                    parse_fallback: true,
                },
            },
            Err(_) => MemoryLogMeta {
                kind: kind_name(&fragment.kind),
                event_type: None,
                tool_call_count: None,
                text_len: Some(fragment.content.len()),
                parse_fallback: true,
            },
        },
        MemoryKind::Unknown => MemoryLogMeta {
            kind: kind_name(&fragment.kind),
            event_type: None,
            tool_call_count: None,
            text_len: Some(fragment.content.len()),
            parse_fallback: false,
        },
    }
}

pub fn summarize_batch_log_meta(fragments: &[MemoryFragment]) -> MemoryBatchLogMeta {
    let mut kind_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut event_type_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut parse_fallback_count = 0;

    for fragment in fragments {
        let meta = fragment_log_meta(fragment);
        *kind_counts.entry(meta.kind).or_insert(0) += 1;
        if let Some(event_type) = meta.event_type {
            *event_type_counts.entry(event_type).or_insert(0) += 1;
        }
        if meta.parse_fallback {
            parse_fallback_count += 1;
        }
    }

    MemoryBatchLogMeta {
        total: fragments.len(),
        kind_counts,
        event_type_counts,
        parse_fallback_count,
    }
}

#[cfg(test)]
mod tests {
    use super::super::memory_constructors::lifecycle_startup_event;
    use super::super::memory_content::{ToolCallRecord, pending_memory};
    use super::*;
    use loom_client::memory::MemoryKind;
    use serde_json::json;

    #[test]
    fn lifecycle_startup_event_meta_contains_event_type() {
        let fragment = lifecycle_startup_event(false);
        let meta = fragment_log_meta(&fragment);
        assert_eq!(meta.kind, "event");
        assert_eq!(meta.event_type.as_deref(), Some("lifecycle.startup"));
        assert!(!meta.parse_fallback);
    }

    #[test]
    fn lifecycle_awakening_event_meta_contains_event_type() {
        let fragment = lifecycle_startup_event(true);
        let meta = fragment_log_meta(&fragment);
        assert_eq!(meta.kind, "event");
        assert_eq!(meta.event_type.as_deref(), Some("lifecycle.awakening"));
        assert!(!meta.parse_fallback);
    }

    #[test]
    fn malformed_event_fragment_uses_fallback() {
        let fragment = pending_memory("{not-json".to_string(), MemoryKind::Event);
        let meta = fragment_log_meta(&fragment);
        assert_eq!(meta.kind, "event");
        assert_eq!(meta.event_type, None);
        assert!(meta.parse_fallback);
    }

    #[test]
    fn action_fragment_reports_tool_call_count() {
        let action = ActionMemoryContent {
            tool_calls: vec![
                ToolCallRecord {
                    id: "call-1".to_string(),
                    tool: "tool_a".to_string(),
                    args: json!({"a": 1}),
                    result: "ok".to_string(),
                },
                ToolCallRecord {
                    id: "call-2".to_string(),
                    tool: "tool_b".to_string(),
                    args: json!({"b": 2}),
                    result: "ok".to_string(),
                },
            ],
        };
        let content = serde_json::to_string(&action).unwrap();
        let fragment = pending_memory(content, MemoryKind::Action);
        let meta = fragment_log_meta(&fragment);
        assert_eq!(meta.kind, "action");
        assert_eq!(meta.tool_call_count, Some(2));
        assert!(!meta.parse_fallback);
    }

    #[test]
    fn batch_summary_aggregates_kinds_and_event_types() {
        let startup = lifecycle_startup_event(false);
        let awakening = lifecycle_startup_event(true);
        let thought = pending_memory(
            serde_json::to_string(&ThoughtContent { text: "hello".to_string() }).unwrap(),
            MemoryKind::Thought,
        );
        let summary = summarize_batch_log_meta(&[startup, awakening, thought]);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.kind_counts.get("event"), Some(&2));
        assert_eq!(summary.kind_counts.get("thought"), Some(&1));
        assert_eq!(summary.event_type_counts.get("lifecycle.startup"), Some(&1));
        assert_eq!(
            summary.event_type_counts.get("lifecycle.awakening"),
            Some(&1)
        );
        assert_eq!(summary.parse_fallback_count, 0);
    }
}
