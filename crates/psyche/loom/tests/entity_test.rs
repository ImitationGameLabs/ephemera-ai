use loom::memory::types::{MemoryFragment, MemoryKind};
use loom::services::memory::entity::memory;
use time::OffsetDateTime;

fn test_fragment(id: i64, content: &str, kind: MemoryKind) -> MemoryFragment {
    MemoryFragment { id, content: content.to_string(), timestamp: OffsetDateTime::now_utc(), kind }
}

#[test]
fn test_memory_fragment_to_model_conversion() {
    let timestamp = OffsetDateTime::now_utc();

    let fragment = MemoryFragment {
        id: 123,
        content: "test content".to_string(),
        timestamp,
        kind: MemoryKind::Event,
    };

    let model: memory::Model = fragment.into();

    assert_eq!(model.id, 123);
    assert_eq!(model.content, "test content");
    assert_eq!(model.timestamp, timestamp);
    assert_eq!(model.kind, "event");
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
    let original_fragment = test_fragment(789, "roundtrip test", MemoryKind::Action);

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
    let kinds = [
        (MemoryKind::Thought, "thought"),
        (MemoryKind::Action, "action"),
        (MemoryKind::Event, "event"),
        (MemoryKind::Unknown, "unknown"),
    ];

    for (kind, expected_tag) in kinds {
        let fragment = test_fragment(1, "content", kind);
        let model: memory::Model = fragment.into();
        assert_eq!(model.kind, expected_tag);
    }
}

#[test]
fn test_memory_kind_deserialization() {
    let timestamp = OffsetDateTime::now_utc();

    let cases = [
        (1, "thought", MemoryKind::Thought),
        (2, "action", MemoryKind::Action),
        (3, "event", MemoryKind::Event),
        (4, "unknown", MemoryKind::Unknown),
        (5, "invalid_kind", MemoryKind::Unknown),
    ];

    for (id, kind_str, expected_kind) in cases {
        let model = memory::Model {
            id,
            content: "content".to_string(),
            timestamp,
            kind: kind_str.to_string(),
        };
        let fragment: MemoryFragment = model.into();
        assert_eq!(fragment.kind, expected_kind);
    }
}

#[test]
fn test_all_memory_kinds_roundtrip() {
    let kinds = [MemoryKind::Thought, MemoryKind::Action, MemoryKind::Event, MemoryKind::Unknown];

    for expected_kind in kinds {
        let fragment = test_fragment(100, "test content", expected_kind.clone());

        let model: memory::Model = fragment.into();
        let recovered: MemoryFragment = model.into();

        assert_eq!(recovered.kind, expected_kind);
    }
}
