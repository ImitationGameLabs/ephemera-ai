use super::{ContextSerialize, MemoryFragment};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use std::collections::VecDeque;

/// New type for unified MemoryFragment serialization
pub struct MemoryFragmentList(Vec<MemoryFragment>);

impl MemoryFragmentList {
    /// Format timestamp for readable display
    fn format_timestamp(&self, timestamp: i64) -> String {
        let datetime = OffsetDateTime::from_unix_timestamp(timestamp)
            .unwrap_or_else(|_| OffsetDateTime::now_utc());
        datetime.format(&Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string())
    }

    /// Serialize a single memory fragment with detailed information
    fn serialize_memory(&self, memory: &MemoryFragment) -> String {
        format!(
            "Memory ID: {}\nCreated: {}\nSource: {}\nImportance: {}/255\nConfidence: {}/255\nTags: {}\nContent: {}",
            memory.id,
            self.format_timestamp(memory.objective_metadata.created_at),
            format!("{}::{}", memory.objective_metadata.source.channel, memory.objective_metadata.source.identifier),
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
    use loom_client::memory::{MemorySource, ObjectiveMetadata, SubjectiveMetadata};

    #[test]
    fn test_memory_fragment_list_serialization() {
        let fragment1 = MemoryFragment {
            id: 1,
            content: "Test perception".to_string(),
            subjective_metadata: SubjectiveMetadata {
                importance: 120,
                confidence: 200,
                tags: vec!["perception".to_string()],
                notes: String::new(),
            },
            objective_metadata: ObjectiveMetadata {
                created_at: OffsetDateTime::now_utc().unix_timestamp(),
                source: MemorySource::dialogue_input("test_source".to_string()),
            },
            associations: Vec::new(),
        };

        let fragment2 = MemoryFragment {
            id: 2,
            content: "test_action: test_details".to_string(),
            subjective_metadata: SubjectiveMetadata {
                importance: 100,
                confidence: 255,
                tags: vec!["activity".to_string(), "test_action".to_string()],
                notes: String::new(),
            },
            objective_metadata: ObjectiveMetadata {
                created_at: OffsetDateTime::now_utc().unix_timestamp(),
                source: MemorySource::action("execution".to_string()),
            },
            associations: Vec::new(),
        };

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