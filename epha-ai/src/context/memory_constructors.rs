//! Convenience constructors for MemoryFragmentBuilder
//!
//! This module provides high-level convenience functions for creating MemoryFragmentBuilders
//! with common patterns used in the Ephemera AI application.

use loom_client::{MemoryFragmentBuilder, memory::MemorySource};
use std::collections::HashMap;

/// Create a builder for a memory fragment from an AI thought with specified reasoning type
pub fn from_reasoning(content: String, reasoning_type: &str) -> MemoryFragmentBuilder {
    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), reasoning_type.to_string());

    let source = MemorySource {
        channel: "thought".to_string(),
        identifier: "self_thought".to_string(),
        metadata,
    };

    MemoryFragmentBuilder::new(content, source)
}

/// Create a builder for a memory fragment from user dialogue input
pub fn from_dialogue_input(content: String, user: &str) -> MemoryFragmentBuilder {
    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), "input".to_string());

    let source = MemorySource {
        channel: "dialogue".to_string(),
        identifier: user.to_string(),
        metadata,
    };

    MemoryFragmentBuilder::new(content, source)
}

/// Create a builder for a memory fragment from dialogue response from AI
pub fn from_dialogue_response(content: String) -> MemoryFragmentBuilder {
    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), "output".to_string());

    let source = MemorySource {
        channel: "dialogue".to_string(),
        identifier: "self".to_string(),
        metadata,
    };

    MemoryFragmentBuilder::new(content, source)
}

/// Create a builder for a memory fragment from an information source
pub fn from_information(content: String, source: &str, source_type: &str) -> MemoryFragmentBuilder {
    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), source_type.to_string());

    let memory_source = MemorySource {
        channel: "information".to_string(),
        identifier: source.to_string(),
        metadata,
    };

    MemoryFragmentBuilder::new(content, memory_source)
}

/// Create a builder for a memory fragment from an AI action
pub fn from_action(content: String, action_type: &str) -> MemoryFragmentBuilder {
    let mut metadata = HashMap::new();
    metadata.insert("type".to_string(), action_type.to_string());

    let source = MemorySource {
        channel: "action".to_string(),
        identifier: "self_action".to_string(),
        metadata,
    };

    MemoryFragmentBuilder::new(content, source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convenience_constructors() {
        let thought = from_reasoning("I need to analyze this".to_string(), "reasoning")
            .importance(180)
            .build();
        assert_eq!(thought.content, "I need to analyze this");
        assert_eq!(thought.objective_metadata.source.channel, "thought");
        assert_eq!(thought.objective_metadata.source.identifier, "self_thought");
        assert_eq!(thought.objective_metadata.source.metadata.get("type"), Some(&"reasoning".to_string()));
        assert_eq!(thought.subjective_metadata.importance, 180);

        let dialogue = from_dialogue_input("Hello world".to_string(), "alice")
            .confidence(200)
            .add_tag("greeting".to_string())
            .build();
        assert_eq!(dialogue.content, "Hello world");
        assert_eq!(dialogue.objective_metadata.source.channel, "dialogue");
        assert_eq!(dialogue.objective_metadata.source.identifier, "alice");
        assert_eq!(dialogue.objective_metadata.source.metadata.get("type"), Some(&"input".to_string()));
        assert_eq!(dialogue.subjective_metadata.confidence, 200);
        assert!(dialogue.subjective_metadata.tags.contains(&"greeting".to_string()));

        let info = from_information("Config loaded".to_string(), "config.json", "file")
            .add_tag("system".to_string())
            .build();
        assert_eq!(info.content, "Config loaded");
        assert_eq!(info.objective_metadata.source.channel, "information");
        assert_eq!(info.objective_metadata.source.identifier, "config.json");
        assert_eq!(info.objective_metadata.source.metadata.get("type"), Some(&"file".to_string()));
        assert!(info.subjective_metadata.tags.contains(&"system".to_string()));

        let action = from_action("Task completed".to_string(), "execution")
            .importance(160)
            .confidence(190)
            .build();
        assert_eq!(action.content, "Task completed");
        assert_eq!(action.objective_metadata.source.channel, "action");
        assert_eq!(action.objective_metadata.source.identifier, "self_action");
        assert_eq!(action.objective_metadata.source.metadata.get("type"), Some(&"execution".to_string()));
        assert_eq!(action.subjective_metadata.importance, 160);
        assert_eq!(action.subjective_metadata.confidence, 190);
    }
}