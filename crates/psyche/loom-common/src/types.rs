use serde::{Deserialize, Serialize};

/// Memory kind - the category of a memory fragment
///
/// Classification based on source and agency:
/// - Thought: AI's internal cognitive processes (reasoning, planning, reflection)
/// - Action: AI's initiated activities (tool calls, execution results)
/// - Event: External information injected into AI context
///   - System events (startup, shutdown, config changes)
///   - Producer events (dialogue, timer, notifications via EventHub)
/// - Unknown: Classification error - should be investigated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    /// AI's internal cognitive processes: reasoning, planning, reflection
    Thought,
    /// AI's initiated activities: tool calls, execution results
    Action,
    /// External information: system events, producer events
    Event,
    /// Unrecognized kind - indicates classification error
    Unknown,
}

impl MemoryKind {
    /// Convert to InfluxDB tag value
    pub fn as_tag(&self) -> &'static str {
        match self {
            MemoryKind::Thought => "thought",
            MemoryKind::Action => "action",
            MemoryKind::Event => "event",
            MemoryKind::Unknown => "unknown",
        }
    }

    /// Parse from string (lowercase only).
    /// Unrecognized values return `Unknown` rather than an error,
    /// because invalid data from the database should still be loadable.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "thought" => MemoryKind::Thought,
            "action" => MemoryKind::Action,
            "event" => MemoryKind::Event,
            _ => MemoryKind::Unknown,
        }
    }
}

impl Default for MemoryKind {
    fn default() -> Self {
        MemoryKind::Event
    }
}

impl std::fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.as_tag())
    }
}

/// MemoryFragment represents a minimal, immutable event in the memory stream.
/// Each fragment is a simple record of something that happened, stored in chronological order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFragment {
    /// Unique identifier (Snowflake-like ID)
    pub id: i64,
    /// JSON content with type-specific structure
    pub content: String,
    /// When this memory was created
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: time::OffsetDateTime,
    /// The kind/category of this memory
    pub kind: MemoryKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_kind_from_str() {
        assert_eq!(MemoryKind::from_str("thought"), MemoryKind::Thought);
        assert_eq!(MemoryKind::from_str("action"), MemoryKind::Action);
        assert_eq!(MemoryKind::from_str("event"), MemoryKind::Event);
        assert_eq!(MemoryKind::from_str("unknown"), MemoryKind::Unknown);
        // Non-lowercase and unrecognized strings return Unknown
        assert_eq!(MemoryKind::from_str("Thought"), MemoryKind::Unknown);
        assert_eq!(MemoryKind::from_str("ACTION"), MemoryKind::Unknown);
        assert_eq!(MemoryKind::from_str("invalid"), MemoryKind::Unknown);
        assert_eq!(MemoryKind::from_str(""), MemoryKind::Unknown);
    }

    #[test]
    fn test_memory_kind_as_tag() {
        assert_eq!(MemoryKind::Thought.as_tag(), "thought");
        assert_eq!(MemoryKind::Action.as_tag(), "action");
        assert_eq!(MemoryKind::Event.as_tag(), "event");
        assert_eq!(MemoryKind::Unknown.as_tag(), "unknown");
    }
}
