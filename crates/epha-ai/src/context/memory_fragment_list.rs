use loom_client::memory::MemoryFragment;
use std::collections::VecDeque;

/// New type for unified MemoryFragment serialization
pub struct MemoryFragmentList(Vec<MemoryFragment>);

impl MemoryFragmentList {
    /// Format datetime for readable display with millisecond precision (3 decimal places)
    fn format_datetime(&self, datetime: time::OffsetDateTime) -> String {
        // Use custom format to limit to 3 decimal places (milliseconds)
        let format = time::format_description::parse(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
        )
        .unwrap();
        datetime.format(&format).unwrap_or_else(|_| "unknown".to_string())
    }

    /// Serialize a single memory fragment with simplified information
    fn serialize_memory(&self, memory: &MemoryFragment) -> String {
        format!(
            "Memory ID: {}\nTimestamp: {}\nKind: {}\nContent: {}",
            memory.id,
            self.format_datetime(memory.timestamp),
            memory.kind,
            memory.content
        )
    }
}

impl From<Vec<MemoryFragment>> for MemoryFragmentList {
    fn from(memories: Vec<MemoryFragment>) -> Self {
        Self(memories)
    }
}

impl From<VecDeque<MemoryFragment>> for MemoryFragmentList {
    fn from(memories: VecDeque<MemoryFragment>) -> Self {
        Self(memories.into())
    }
}

impl MemoryFragmentList {
    pub fn serialize(&self) -> String {
        if self.0.is_empty() {
            return "No memories found.".to_string();
        }

        let memories_text: Vec<String> =
            self.0.iter().map(|memory| self.serialize_memory(memory)).collect();

        format!("Found {} memories:\n\n{}", self.0.len(), memories_text.join("\n---\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_client::memory::{MemoryFragment, MemoryKind};

    #[test]
    fn test_memory_fragment_list_serialization() {
        let fragment1 = MemoryFragment {
            id: 1,
            content: r#"{"type":"input","text":"Test perception"}"#.to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind: MemoryKind::Event,
        };

        let fragment2 = MemoryFragment {
            id: 2,
            content: r#"{"type":"execution","action":"test_action: test_details"}"#.to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind: MemoryKind::Action,
        };

        let memories = vec![fragment1, fragment2];
        let memory_list = MemoryFragmentList::from(memories);

        let serialized = memory_list.serialize();

        assert!(serialized.contains("Found 2 memories"));
        assert!(serialized.contains("Memory ID:"));
        assert!(serialized.contains("Timestamp:"));
        assert!(serialized.contains("Kind:"));
        assert!(serialized.contains("Content:"));
        assert!(serialized.contains("---"));
        assert!(serialized.contains("Test perception"));
        assert!(serialized.contains("test_action: test_details"));
    }

    #[test]
    fn test_memory_fragment_list_empty() {
        let empty_list = MemoryFragmentList::from(vec![]);
        let serialized = empty_list.serialize();
        assert_eq!(serialized, "No memories found.");
    }
}
