use loom::memory::types::{MemoryFragment, MemorySource, SubjectiveMetadata, ObjectiveMetadata};
use time::OffsetDateTime;

#[test]
fn test_new_memory_format_serialization() {
    // Create a test memory fragment with new structure
    let memory = MemoryFragment {
        id: 0, // Will be set by database
        content: "Test memory with new architecture".to_string(),
        subjective_metadata: SubjectiveMetadata {
            importance: 150,
            confidence: 200,
            tags: vec!["test".to_string(), "new_format".to_string()],
            notes: "Test note for new memory format".to_string(),
        },
        objective_metadata: ObjectiveMetadata {
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            source: MemorySource {
                channel: "dialogue".to_string(),
                identifier: "test_user".to_string(),
                metadata: [("type".to_string(), "input".to_string())].into_iter().collect(),
            },
        },
        associations: vec![1, 2, 3],
    };

    // Test JSON serialization
    let source_json = serde_json::to_string(&memory.objective_metadata.source).unwrap();
    println!("Serialized MemorySource: {}", source_json);

    // Test JSON deserialization
    let deserialized_source: MemorySource = serde_json::from_str(&source_json).unwrap();
    assert_eq!(deserialized_source.channel, "dialogue");
    assert_eq!(deserialized_source.identifier, "test_user");
    assert_eq!(deserialized_source.metadata.get("type"), Some(&"input".to_string()));

    // Test tags serialization
    let tags_json = serde_json::to_string(&memory.subjective_metadata.tags).unwrap();
    let deserialized_tags: Vec<String> = serde_json::from_str(&tags_json).unwrap();
    assert_eq!(memory.subjective_metadata.tags, deserialized_tags);

    // Test associations serialization
    let associations_json = serde_json::to_string(&memory.associations).unwrap();
    let deserialized_associations: Vec<i64> = serde_json::from_str(&associations_json).unwrap();
    assert_eq!(memory.associations, deserialized_associations);
}

#[test]
fn test_memory_source_direct_construction() {
    // Test direct MemorySource construction
    let dialogue_input = MemorySource {
        channel: "dialogue".to_string(),
        identifier: "alice".to_string(),
        metadata: [("type".to_string(), "input".to_string())].into_iter().collect(),
    };
    assert_eq!(dialogue_input.channel, "dialogue");
    assert_eq!(dialogue_input.identifier, "alice");
    assert_eq!(dialogue_input.metadata.get("type"), Some(&"input".to_string()));

    let dialogue_response = MemorySource {
        channel: "dialogue".to_string(),
        identifier: "self".to_string(),
        metadata: [("type".to_string(), "output".to_string())].into_iter().collect(),
    };
    assert_eq!(dialogue_response.channel, "dialogue");
    assert_eq!(dialogue_response.identifier, "self");
    assert_eq!(dialogue_response.metadata.get("type"), Some(&"output".to_string()));

    let information = MemorySource {
        channel: "information".to_string(),
        identifier: "config.json".to_string(),
        metadata: [("type".to_string(), "file".to_string())].into_iter().collect(),
    };
    assert_eq!(information.channel, "information");
    assert_eq!(information.identifier, "config.json");
    assert_eq!(information.metadata.get("type"), Some(&"file".to_string()));

    let thought = MemorySource {
        channel: "thought".to_string(),
        identifier: "self_thought".to_string(),
        metadata: [("type".to_string(), "reasoning".to_string())].into_iter().collect(),
    };
    assert_eq!(thought.channel, "thought");
    assert_eq!(thought.identifier, "self_thought");
    assert_eq!(thought.metadata.get("type"), Some(&"reasoning".to_string()));

    let action = MemorySource {
        channel: "action".to_string(),
        identifier: "self_action".to_string(),
        metadata: [("type".to_string(), "execution".to_string())].into_iter().collect(),
    };
    assert_eq!(action.channel, "action");
    assert_eq!(action.identifier, "self_action");
    assert_eq!(action.metadata.get("type"), Some(&"execution".to_string()));
}

#[test]
fn test_memory_source_display() {
    let source = MemorySource {
        channel: "dialogue".to_string(),
        identifier: "bob".to_string(),
        metadata: [("type".to_string(), "input".to_string())].into_iter().collect(),
    };
    let display_str = format!("{}", source);
    assert_eq!(display_str, "[dialogue:input] bob");

    let information = MemorySource {
        channel: "information".to_string(),
        identifier: "web".to_string(),
        metadata: [("type".to_string(), "url".to_string())].into_iter().collect(),
    };
    let display_str = format!("{}", information);
    assert_eq!(display_str, "[information:url] web");
}