use loom::memory::builder::MemoryFragmentBuilder;
use loom::memory::types::{MemoryFragment, MemoryKind};
use loom::services::memory::entity::memory;
use time::OffsetDateTime;

#[test]
fn test_memory_fragment_to_model_conversion() {
    let timestamp = OffsetDateTime::now_utc();

    let fragment = MemoryFragment {
        id: 123,
        content: "test content".to_string(),
        timestamp,
        kind: MemoryKind::Message,
    };

    let model: memory::Model = fragment.into();

    assert_eq!(model.id, 123);
    assert_eq!(model.content, "test content");
    assert_eq!(model.timestamp, timestamp);
    assert_eq!(model.kind, "message");
}

#[test]
fn test_model_to_memory_fragment_conversion() {
    let timestamp = OffsetDateTime::now_utc();

    let model = memory::Model {
        id: 456,
        content: "model content".to_string(),
        timestamp,
        kind: "thought".to_string(),
    };

    let fragment: MemoryFragment = model.into();

    assert_eq!(fragment.id, 456);
    assert_eq!(fragment.content, "model content");
    assert_eq!(fragment.timestamp, timestamp);
    assert_eq!(fragment.kind, MemoryKind::Thought);
}

#[test]
fn test_memory_fragment_model_roundtrip() {
    let original_fragment =
        MemoryFragmentBuilder::new("roundtrip test".to_string(), MemoryKind::Action)
            .id(789)
            .build();

    // Fragment -> Model
    let model: memory::Model = original_fragment.clone().into();

    // Model -> Fragment
    let recovered_fragment: MemoryFragment = model.into();

    // All fields should be preserved
    assert_eq!(recovered_fragment.id, 789);
    assert_eq!(recovered_fragment.content, "roundtrip test");
    assert_eq!(recovered_fragment.kind, MemoryKind::Action);
}

#[test]
fn test_memory_kind_serialization() {
    // Test Thought
    let fragment =
        MemoryFragmentBuilder::new("thinking...".to_string(), MemoryKind::Thought).build();
    let model: memory::Model = fragment.into();
    assert_eq!(model.kind, "thought");

    // Test Action
    let fragment = MemoryFragmentBuilder::new("acting...".to_string(), MemoryKind::Action).build();
    let model: memory::Model = fragment.into();
    assert_eq!(model.kind, "action");

    // Test Message
    let fragment =
        MemoryFragmentBuilder::new("messaging...".to_string(), MemoryKind::Message).build();
    let model: memory::Model = fragment.into();
    assert_eq!(model.kind, "message");
}

#[test]
fn test_memory_kind_deserialization() {
    let timestamp = OffsetDateTime::now_utc();

    // Test Thought
    let model = memory::Model {
        id: 1,
        content: "content".to_string(),
        timestamp,
        kind: "thought".to_string(),
    };
    let fragment: MemoryFragment = model.into();
    assert_eq!(fragment.kind, MemoryKind::Thought);

    // Test Action
    let model = memory::Model {
        id: 2,
        content: "content".to_string(),
        timestamp,
        kind: "action".to_string(),
    };
    let fragment: MemoryFragment = model.into();
    assert_eq!(fragment.kind, MemoryKind::Action);

    // Test Message (default for unknown)
    let model = memory::Model {
        id: 3,
        content: "content".to_string(),
        timestamp,
        kind: "unknown".to_string(),
    };
    let fragment: MemoryFragment = model.into();
    assert_eq!(fragment.kind, MemoryKind::Message);
}

#[test]
fn test_all_memory_kinds_roundtrip() {
    let kinds = [MemoryKind::Thought, MemoryKind::Action, MemoryKind::Message];

    for expected_kind in kinds {
        let fragment =
            MemoryFragmentBuilder::new("test content".to_string(), expected_kind.clone())
                .id(100)
                .build();

        let model: memory::Model = fragment.into();
        let recovered: MemoryFragment = model.into();

        assert_eq!(recovered.kind, expected_kind);
    }
}
