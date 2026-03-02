use serde::{Deserialize, Serialize};

/// Memory kind - the category of a memory fragment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoryKind {
    /// AI's internal thinking: reasoning, planning, reflection
    Thought,
    /// Tool/function calls: actions taken by the AI
    Action,
    /// External triggers: messages, events, notifications
    Message,
}

impl MemoryKind {
    /// Convert to InfluxDB tag value
    pub fn as_tag(&self) -> &'static str {
        match self {
            MemoryKind::Thought => "thought",
            MemoryKind::Action => "action",
            MemoryKind::Message => "message",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "thought" => MemoryKind::Thought,
            "action" => MemoryKind::Action,
            _ => MemoryKind::Message,
        }
    }
}

impl Default for MemoryKind {
    fn default() -> Self {
        MemoryKind::Message
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
