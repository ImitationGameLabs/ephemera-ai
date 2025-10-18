use super::{ContextSerialize, MemoryFragment};
use time::OffsetDateTime;
use time::format_description;
use std::collections::VecDeque;

/// New type for unified MemoryFragment serialization
pub struct MemoryFragmentList(Vec<MemoryFragment>);

impl MemoryFragmentList {
    /// Format timestamp for readable display
    fn format_timestamp(&self, timestamp: i64) -> String {
        let datetime = OffsetDateTime::from_unix_timestamp(timestamp)
            .unwrap_or_else(|_| OffsetDateTime::now_utc());

        let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
            .unwrap_or_else(|_| format_description::parse("[year]-[month]-[day]T[hour]:[minute]:[second]Z").unwrap());

        datetime.format(&format)
            .unwrap_or_else(|_| timestamp.to_string())
    }

    /// Serialize a single memory fragment with detailed information
    fn serialize_memory(&self, memory: &MemoryFragment) -> String {
        format!(
            "Memory ID: {}\nCreated: {}\nSource: {}\nImportance: {}/255\nConfidence: {}/255\nTags: {}\nContent: {}",
            memory.id,
            self.format_timestamp(memory.objective_metadata.created_at),
            memory.objective_metadata.source.channel,
            memory.subjective_metadata.importance,
            memory.subjective_metadata.confidence,
            memory.subjective_metadata.tags.join(", "),
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

        let memories_text: Vec<String> = self.0
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
    use epha_memory::{MemorySource, MemoryFragmentBuilder};

    #[test]
    fn test_memory_fragment_list_serialization() {
        let fragment1 = MemoryFragmentBuilder::new()
            .content("Test perception".to_string())
            .importance(120)
            .confidence(200)
            .add_tag("perception".to_string())
            .source(MemorySource::dialogue_input("test_source".to_string()))
            .build();

        let fragment2 = MemoryFragmentBuilder::new()
            .content("test_action: test_details".to_string())
            .importance(100)
            .confidence(255)
            .add_tag("activity".to_string())
            .add_tag("test_action".to_string())
            .source(MemorySource::action("execution".to_string()))
            .build();

        let memories = vec![fragment1, fragment2];
        let memory_list = MemoryFragmentList::from(memories);

        let serialized = memory_list.serialize();

        // Check that the serialization contains expected elements
        assert!(serialized.contains("Found 2 memories"));
        assert!(serialized.contains("Memory ID:"));
        assert!(serialized.contains("Created:"));
        assert!(serialized.contains("Source:"));
        assert!(serialized.contains("Importance:"));
        assert!(serialized.contains("Confidence:"));
        assert!(serialized.contains("Tags:"));
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