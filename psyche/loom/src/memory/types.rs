use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// MemoryFragment represents a minimal, immutable event in the memory stream.
/// Each fragment is a simple record of something that happened, stored in chronological order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFragment {
    /// Unique identifier for this memory fragment
    pub id: i64,
    /// The content/text of the memory
    pub content: String,
    /// When this memory was created
    #[serde(with = "time::serde::iso8601")]
    pub timestamp: time::OffsetDateTime,
    /// Source of the memory, indicating its origin
    pub source: MemorySource,
}

/// Represents the origin of a memory fragment with channel-based design.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySource {
    /// Channel category: "dialogue", "information", "thought", "action"
    pub channel: String,
    /// Unique identifier for the specific source instance
    pub identifier: String,
    /// Additional metadata for rich source information
    pub metadata: HashMap<String, String>,
}

impl Default for MemorySource {
    fn default() -> Self {
        Self {
            channel: "unknown".to_string(),
            identifier: "unknown".to_string(),
            metadata: HashMap::new(),
        }
    }
}

impl fmt::Display for MemorySource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let type_str = self
            .metadata
            .get("type")
            .map(|t| format!(":{}", t))
            .unwrap_or_default();
        write!(f, "[{}{}] {}", self.channel, type_str, self.identifier)
    }
}
