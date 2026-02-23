use epha_agent::context::ContextSerialize;
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
        datetime
            .format(&format)
            .unwrap_or_else(|_| "unknown".to_string())
    }

    /// Serialize a single memory fragment with simplified information
    fn serialize_memory(&self, memory: &MemoryFragment) -> String {
        format!(
            "Memory ID: {}\nTimestamp: {}\nSource: {}\nContent: {}",
            memory.id,
            self.format_datetime(memory.timestamp),
            format!("{}::{}", memory.source.channel, memory.source.identifier),
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

impl ContextSerialize for MemoryFragmentList {
    fn serialize(&self) -> String {
        if self.0.is_empty() {
            return "No memories found.".to_string();
        }

        let memories_text: Vec<String> = self
            .0
            .iter()
            .map(|memory| self.serialize_memory(memory))
            .collect();

        format!(
            "Found {} memories:\n\n{}",
            self.0.len(),
            memories_text.join("\n---\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_client::{MemoryFragmentBuilder, memory::MemorySource};

    #[test]
    fn test_memory_fragment_list_serialization() {
        let source1 = MemorySource {
            channel: "dialogue".to_string(),
            identifier: "test_source".to_string(),
            metadata: [("type".to_string(), "input".to_string())]
                .into_iter()
                .collect(),
        };

        let mut fragment1 =
            MemoryFragmentBuilder::new("Test perception".to_string(), source1).build();

        // Override the ID for test purposes
        fragment1.id = 1;

        let source2 = MemorySource {
            channel: "action".to_string(),
            identifier: "self_action".to_string(),
            metadata: [("type".to_string(), "execution".to_string())]
                .into_iter()
                .collect(),
        };

        let mut fragment2 =
            MemoryFragmentBuilder::new("test_action: test_details".to_string(), source2).build();

        // Override the ID for test purposes
        fragment2.id = 2;

        let memories = vec![fragment1, fragment2];
        let memory_list = MemoryFragmentList::from(memories);

        let serialized = memory_list.serialize();

        // Check that the serialization contains expected elements
        assert!(serialized.contains("Found 2 memories"));
        assert!(serialized.contains("Memory ID:"));
        assert!(serialized.contains("Timestamp:"));
        assert!(serialized.contains("Source:"));
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
