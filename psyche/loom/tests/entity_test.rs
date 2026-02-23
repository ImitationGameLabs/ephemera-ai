use loom::memory::builder::MemoryFragmentBuilder;
use loom::memory::types::{MemoryFragment, MemorySource};
use loom::services::memory::entity::memory;
use std::collections::HashMap;
use time::OffsetDateTime;

fn test_source(channel: &str, identifier: &str) -> MemorySource {
    MemorySource {
        channel: channel.to_string(),
        identifier: identifier.to_string(),
        metadata: HashMap::new(),
    }
}

#[test]
fn test_memory_fragment_to_model_conversion() {
    let source = test_source("dialogue", "alice");
    let timestamp = OffsetDateTime::now_utc();

    let fragment = MemoryFragment {
        id: 123,
        content: "test content".to_string(),
        timestamp,
        source: source.clone(),
    };

    let model: memory::Model = fragment.into();

    assert_eq!(model.id, 0); // ID is set to 0 during conversion (will be auto-generated)
    assert_eq!(model.content, "test content");
    assert_eq!(model.timestamp, timestamp);
    // Source should be serialized to JSON
    let deserialized_source: MemorySource = serde_json::from_str(&model.source).unwrap();
    assert_eq!(deserialized_source.channel, "dialogue");
    assert_eq!(deserialized_source.identifier, "alice");
}

#[test]
fn test_model_to_memory_fragment_conversion() {
    let source = MemorySource {
        channel: "information".to_string(),
        identifier: "api".to_string(),
        metadata: [("type".to_string(), "test".to_string())]
            .into_iter()
            .collect(),
    };
    let source_json = serde_json::to_string(&source).unwrap();
    let timestamp = OffsetDateTime::now_utc();

    let model = memory::Model {
        id: 456,
        content: "model content".to_string(),
        timestamp,
        source: source_json,
    };

    let fragment: MemoryFragment = model.into();

    assert_eq!(fragment.id, 456);
    assert_eq!(fragment.content, "model content");
    assert_eq!(fragment.timestamp, timestamp);
    assert_eq!(fragment.source.channel, "information");
    assert_eq!(fragment.source.identifier, "api");
    assert_eq!(
        fragment.source.metadata.get("type"),
        Some(&"test".to_string())
    );
}

#[test]
fn test_memory_fragment_model_roundtrip() {
    let original_fragment = MemoryFragmentBuilder::new(
        "roundtrip test".to_string(),
        test_source("thought", "reflection"),
    )
    .id(789)
    .build();

    // Fragment -> Model
    let model: memory::Model = original_fragment.clone().into();

    // Model -> Fragment
    let recovered_fragment: MemoryFragment = model.into();

    // Note: The original ID is lost during fragment->model conversion (set to 0)
    // But content, timestamp, and source should be preserved
    assert_eq!(recovered_fragment.content, "roundtrip test");
    assert_eq!(recovered_fragment.source.channel, "thought");
    assert_eq!(recovered_fragment.source.identifier, "reflection");
}

#[test]
fn test_memory_source_serialization() {
    let source = MemorySource {
        channel: "action".to_string(),
        identifier: "tool-execution".to_string(),
        metadata: [
            ("type".to_string(), "shell".to_string()),
            ("exit_code".to_string(), "0".to_string()),
        ]
        .into_iter()
        .collect(),
    };

    let json = serde_json::to_string(&source).unwrap();
    assert!(json.contains("\"channel\":\"action\""));
    assert!(json.contains("\"identifier\":\"tool-execution\""));
    assert!(json.contains("\"type\":\"shell\""));

    let deserialized: MemorySource = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.channel, "action");
    assert_eq!(deserialized.identifier, "tool-execution");
    assert_eq!(
        deserialized.metadata.get("type"),
        Some(&"shell".to_string())
    );
    assert_eq!(
        deserialized.metadata.get("exit_code"),
        Some(&"0".to_string())
    );
}

#[test]
fn test_model_source_with_complex_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("user_id".to_string(), "user-123".to_string());
    metadata.insert("session_id".to_string(), "sess-456".to_string());
    metadata.insert("environment".to_string(), "production".to_string());

    let source = MemorySource {
        channel: "dialogue".to_string(),
        identifier: "chat-message".to_string(),
        metadata,
    };

    let fragment = MemoryFragmentBuilder::new("complex metadata test".to_string(), source).build();

    let model: memory::Model = fragment.into();
    let recovered: MemoryFragment = model.into();

    assert_eq!(recovered.source.metadata.len(), 3);
    assert_eq!(
        recovered.source.metadata.get("user_id"),
        Some(&"user-123".to_string())
    );
    assert_eq!(
        recovered.source.metadata.get("session_id"),
        Some(&"sess-456".to_string())
    );
    assert_eq!(
        recovered.source.metadata.get("environment"),
        Some(&"production".to_string())
    );
}
