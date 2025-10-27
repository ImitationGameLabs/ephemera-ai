use super::types::{MemoryFragment, MemorySource, SubjectiveMetadata, ObjectiveMetadata};
use time::OffsetDateTime;

/// Builder for creating MemoryFragment instances with flexible configuration
pub struct MemoryFragmentBuilder {
    fragment: MemoryFragment,
}

impl MemoryFragmentBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            fragment: MemoryFragment {
                id: 0, // Will be set by database when inserted
                content: String::new(),
                subjective_metadata: SubjectiveMetadata {
                    importance: 100,
                    confidence: 255,
                    tags: Vec::new(),
                    notes: String::new(),
                },
                objective_metadata: ObjectiveMetadata {
                    created_at: OffsetDateTime::now_utc().unix_timestamp(),
                    source: MemorySource::information("builder".to_string(), "default".to_string()),
                },
                associations: Vec::new(),
            },
        }
    }

    /// Set the content of the memory fragment
    pub fn content(mut self, content: String) -> Self {
        self.fragment.content = content;
        self
    }

    /// Set the importance level (0-255)
    pub fn importance(mut self, importance: u8) -> Self {
        self.fragment.subjective_metadata.importance = importance;
        self
    }

    /// Set the confidence level (0-255)
    pub fn confidence(mut self, confidence: u8) -> Self {
        self.fragment.subjective_metadata.confidence = confidence;
        self
    }

    /// Add a tag to the memory fragment
    pub fn add_tag(mut self, tag: String) -> Self {
        self.fragment.subjective_metadata.tags.push(tag);
        self
    }

    /// Set the source of the memory fragment
    pub fn source(mut self, source: MemorySource) -> Self {
        self.fragment.objective_metadata.source = source;
        self
    }

    /// Set notes for the memory fragment
    pub fn notes(mut self, notes: String) -> Self {
        self.fragment.subjective_metadata.notes = notes;
        self
    }

    /// Add an association to the memory fragment
    pub fn add_association(mut self, association: i64) -> Self {
        self.fragment.associations.push(association);
        self
    }

    /// Build the final MemoryFragment
    pub fn build(self) -> MemoryFragment {
        self.fragment
    }
}

impl Default for MemoryFragmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::MemorySource;

    #[test]
    fn test_builder_basic_functionality() {
        // Test that the basic builder functionality works
        let fragment = MemoryFragmentBuilder::new()
            .content("test content".to_string())
            .importance(150)
            .confidence(200)
            .add_tag("test".to_string())
            .source(MemorySource::information("test".to_string(), "unit".to_string()))
            .build();

        assert_eq!(fragment.content, "test content");
        assert_eq!(fragment.subjective_metadata.importance, 150);
        assert_eq!(fragment.subjective_metadata.confidence, 200);
        assert!(fragment.subjective_metadata.tags.contains(&"test".to_string()));
        assert_eq!(fragment.objective_metadata.source.channel, "information");
    }

    #[test]
    fn test_memory_source_creation() {
        let dialogue_source = MemorySource::dialogue_input("alice".to_string());
        assert_eq!(dialogue_source.channel, "dialogue");
        assert_eq!(dialogue_source.identifier, "alice");
        assert_eq!(dialogue_source.metadata.get("type"), Some(&"input".to_string()));

        let info_source = MemorySource::information("config.json".to_string(), "file".to_string());
        assert_eq!(info_source.channel, "information");
        assert_eq!(info_source.identifier, "config.json");
        assert_eq!(info_source.metadata.get("type"), Some(&"file".to_string()));
    }

    #[test]
    fn test_memory_source_display() {
        use std::collections::HashMap;

        let source = MemorySource::dialogue_input("bob".to_string());
        assert_eq!(format!("{}", source), "[dialogue:input] bob");

        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "web".to_string());
        let custom_source = MemorySource {
            channel: "information".to_string(),
            identifier: "example.com".to_string(),
            metadata,
        };
        assert_eq!(format!("{}", custom_source), "[information:web] example.com");
    }
}