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
        let thought = from_reasoning("I need to analyze this".to_string(), "reasoning").build();
        assert_eq!(thought.content, "I need to analyze this");
        assert_eq!(thought.source.channel, "thought");
        assert_eq!(thought.source.identifier, "self_thought");
        assert_eq!(
            thought.source.metadata.get("type"),
            Some(&"reasoning".to_string())
        );

        let dialogue = from_dialogue_input("Hello world".to_string(), "alice").build();
        assert_eq!(dialogue.content, "Hello world");
        assert_eq!(dialogue.source.channel, "dialogue");
        assert_eq!(dialogue.source.identifier, "alice");
        assert_eq!(
            dialogue.source.metadata.get("type"),
            Some(&"input".to_string())
        );

        let info = from_information("Config loaded".to_string(), "config.json", "file").build();
        assert_eq!(info.content, "Config loaded");
        assert_eq!(info.source.channel, "information");
        assert_eq!(info.source.identifier, "config.json");
        assert_eq!(info.source.metadata.get("type"), Some(&"file".to_string()));

        let action = from_action("Task completed".to_string(), "execution").build();
        assert_eq!(action.content, "Task completed");
        assert_eq!(action.source.channel, "action");
        assert_eq!(action.source.identifier, "self_action");
        assert_eq!(
            action.source.metadata.get("type"),
            Some(&"execution".to_string())
        );
    }
}
