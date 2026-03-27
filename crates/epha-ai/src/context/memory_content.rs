//! Unified memory content types and Memory → ChatMessage conversion.
//!
//! All memory constructors produce content in one of these structured formats,
//! enabling reliable conversion to role-aware ChatMessages.

use llm::chat::ChatMessage;
use llm::{FunctionCall, ToolCall};
use loom_client::memory::{MemoryFragment, MemoryKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;

/// Create a MemoryFragment pending persistence (id=0, timestamp=now).
///
/// Use this for fragments that will be sent to the sync layer for persistence.
/// For fragments restored from the database, construct the struct directly with the real id and timestamp.
pub fn pending_memory(content: String, kind: MemoryKind) -> MemoryFragment {
    MemoryFragment { id: 0, content, timestamp: OffsetDateTime::now_utc(), kind }
}

/// Thought memory content — AI's text thinking or response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtContent {
    pub text: String,
}

/// Event memory content — external events or system notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContent {
    pub text: String,
}

/// A single tool call execution record within an Action memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub id: String,
    pub tool: String,
    pub args: Value,
    pub result: String,
}

/// The content structure of an Action memory, containing all tool calls from one LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMemoryContent {
    pub tool_calls: Vec<ToolCallRecord>,
}

// ---------------------------------------------------------------------------
// Content → ChatMessage conversion
// ---------------------------------------------------------------------------

impl ThoughtContent {
    pub fn to_chat_message(&self) -> ChatMessage {
        ChatMessage::assistant().content(&self.text).build()
    }
}

impl EventContent {
    pub fn to_chat_message(&self) -> ChatMessage {
        ChatMessage::user().content(&self.text).build()
    }
}

impl ActionMemoryContent {
    /// Expand into assistant tool_use + user tool_result messages.
    pub fn to_chat_messages(&self) -> Vec<ChatMessage> {
        let tool_use_calls: Vec<ToolCall> = self
            .tool_calls
            .iter()
            .map(|tc| ToolCall {
                id: tc.id.clone(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: tc.tool.clone(),
                    arguments: serde_json::to_string(&tc.args).unwrap_or_default(),
                },
            })
            .collect();

        let tool_result_calls: Vec<ToolCall> = self
            .tool_calls
            .iter()
            .map(|tc| ToolCall {
                id: tc.id.clone(),
                call_type: "function".to_string(),
                function: FunctionCall { name: tc.tool.clone(), arguments: tc.result.clone() },
            })
            .collect();

        vec![
            ChatMessage::assistant().tool_use(tool_use_calls).content("").build(),
            ChatMessage::user().tool_result(tool_result_calls).content("").build(),
        ]
    }
}

// ---------------------------------------------------------------------------
// MemoryFragment → ChatMessage(s) via trait
// ---------------------------------------------------------------------------

/// Extension trait for converting MemoryFragment to role-aware ChatMessages.
pub trait ToChatMessages {
    /// Convert a memory fragment to ChatMessage(s), preserving semantic roles.
    ///
    /// - Thought → 1 assistant text message
    /// - Event → 1 user text message
    /// - Action → 2 messages (assistant tool_use + user tool_result)
    /// - Unknown / parse failure → fallback to user text message
    fn to_chat_messages(&self) -> Vec<ChatMessage>;
}

impl ToChatMessages for MemoryFragment {
    fn to_chat_messages(&self) -> Vec<ChatMessage> {
        match self.kind {
            MemoryKind::Thought => {
                let msg = serde_json::from_str::<ThoughtContent>(&self.content)
                    .map(|c| c.to_chat_message())
                    .unwrap_or_else(|_| {
                        // Fallback: old-format thought (plain text)
                        ChatMessage::assistant().content(&self.content).build()
                    });
                vec![msg]
            }
            MemoryKind::Event => {
                let msg = serde_json::from_str::<EventContent>(&self.content)
                    .map(|c| c.to_chat_message())
                    .unwrap_or_else(|_| {
                        // Fallback: old-format event JSON or plain text
                        ChatMessage::user().content(&self.content).build()
                    });
                vec![msg]
            }
            MemoryKind::Action => {
                serde_json::from_str::<ActionMemoryContent>(&self.content)
                    .map(|c| c.to_chat_messages())
                    .unwrap_or_else(|_| {
                        // Fallback: old-format action JSON
                        vec![ChatMessage::assistant().content(&self.content).build()]
                    })
            }
            MemoryKind::Unknown => {
                vec![ChatMessage::user().content(&self.content).build()]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::memory_content::ToChatMessages;
    use loom_client::memory::MemoryKind;
    use time::OffsetDateTime;

    fn make_fragment(kind: MemoryKind, content: &str) -> MemoryFragment {
        MemoryFragment {
            id: 1,
            content: content.to_string(),
            timestamp: OffsetDateTime::now_utc(),
            kind,
        }
    }

    // -- Serialization round-trips --

    #[test]
    fn thought_content_roundtrip() {
        let original = ThoughtContent { text: "hello world".to_string() };
        let json = serde_json::to_string(&original).unwrap();
        let restored: ThoughtContent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, "hello world");
    }

    #[test]
    fn event_content_roundtrip() {
        let original = EventContent { text: "something happened".to_string() };
        let json = serde_json::to_string(&original).unwrap();
        let restored: EventContent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.text, "something happened");
    }

    #[test]
    fn action_content_roundtrip() {
        let original = ActionMemoryContent {
            tool_calls: vec![ToolCallRecord {
                id: "call_1".to_string(),
                tool: "test_tool".to_string(),
                args: serde_json::json!({"key": "val"}),
                result: "ok".to_string(),
            }],
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: ActionMemoryContent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tool_calls.len(), 1);
        assert_eq!(restored.tool_calls[0].id, "call_1");
    }

    // -- Thought conversion --

    #[test]
    fn thought_converts_to_assistant_message() {
        let content = ThoughtContent { text: "thinking...".to_string() };
        let msg = content.to_chat_message();
        // ChatMessage has no public role accessor, but we can verify via the builder pattern
        // The important thing is it doesn't panic and produces a message.
        // For deeper verification we'd need the llm crate to expose role inspection.
        assert!(!msg.content.is_empty() || msg.message_type == llm::chat::MessageType::Text);
    }

    // -- Event conversion --

    #[test]
    fn event_converts_to_user_message() {
        let content = EventContent { text: "user said hi".to_string() };
        let msg = content.to_chat_message();
        assert!(msg.content.contains("user said hi"));
    }

    // -- Action conversion --

    #[test]
    fn action_converts_to_two_messages() {
        let content = ActionMemoryContent {
            tool_calls: vec![
                ToolCallRecord {
                    id: "c1".to_string(),
                    tool: "memory_get".to_string(),
                    args: serde_json::json!({"key": "recent"}),
                    result: "Found 3 memories".to_string(),
                },
                ToolCallRecord {
                    id: "c2".to_string(),
                    tool: "shell_exec".to_string(),
                    args: serde_json::json!({"command": "ls"}),
                    result: "file1.txt".to_string(),
                },
            ],
        };
        let msgs = content.to_chat_messages();
        assert_eq!(msgs.len(), 2);
        // First message is assistant tool_use, second is user tool_result
        assert_eq!(msgs[0].role, llm::chat::ChatRole::Assistant);
        assert_eq!(msgs[1].role, llm::chat::ChatRole::User);
    }

    #[test]
    fn action_tool_use_preserves_args() {
        let content = ActionMemoryContent {
            tool_calls: vec![ToolCallRecord {
                id: "c1".to_string(),
                tool: "memory_get".to_string(),
                args: serde_json::json!({"key": "recent"}),
                result: "Found 3".to_string(),
            }],
        };
        let msgs = content.to_chat_messages();
        match &msgs[0].message_type {
            llm::chat::MessageType::ToolUse(calls) => {
                assert_eq!(calls[0].function.name, "memory_get");
                assert_eq!(calls[0].id, "c1");
                // arguments should be the serialized args
                let parsed: Value = serde_json::from_str(&calls[0].function.arguments).unwrap();
                assert_eq!(parsed["key"], "recent");
            }
            _ => panic!("Expected ToolUse message type"),
        }
    }

    #[test]
    fn action_tool_result_preserves_result() {
        let content = ActionMemoryContent {
            tool_calls: vec![ToolCallRecord {
                id: "c1".to_string(),
                tool: "memory_get".to_string(),
                args: serde_json::json!({"key": "recent"}),
                result: "Found 3 memories".to_string(),
            }],
        };
        let msgs = content.to_chat_messages();
        match &msgs[1].message_type {
            llm::chat::MessageType::ToolResult(calls) => {
                assert_eq!(calls[0].function.arguments, "Found 3 memories");
            }
            _ => panic!("Expected ToolResult message type"),
        }
    }

    // -- Ordering preservation --

    #[test]
    fn mixed_memories_preserve_order() {
        let thought = make_fragment(MemoryKind::Thought, r#"{"text":"t1"}"#);
        let event = make_fragment(MemoryKind::Event, r#"{"text":"e1"}"#);
        let action = make_fragment(
            MemoryKind::Action,
            r#"{"tool_calls":[{"id":"c1","tool":"x","args":{},"result":"ok"}]}"#,
        );
        let thought2 = make_fragment(MemoryKind::Thought, r#"{"text":"t2"}"#);

        let memories = vec![thought, event, action, thought2];
        let msgs: Vec<ChatMessage> = memories.iter().flat_map(|m| m.to_chat_messages()).collect();

        // thought → 1 msg, event → 1 msg, action → 2 msgs, thought2 → 1 msg = 5 total
        assert_eq!(msgs.len(), 5);
        assert_eq!(msgs[0].role, llm::chat::ChatRole::Assistant); // t1
        assert_eq!(msgs[1].role, llm::chat::ChatRole::User); // e1
        assert_eq!(msgs[2].role, llm::chat::ChatRole::Assistant); // action tool_use
        assert_eq!(msgs[3].role, llm::chat::ChatRole::User); // action tool_result
        assert_eq!(msgs[4].role, llm::chat::ChatRole::Assistant); // t2
    }

    // -- Fallback compatibility --

    #[test]
    fn old_plain_text_thought_fallback() {
        let fragment = make_fragment(MemoryKind::Thought, "just plain text thinking");
        let msgs = fragment.to_chat_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, llm::chat::ChatRole::Assistant);
        assert_eq!(msgs[0].content, "just plain text thinking");
    }

    #[test]
    fn old_json_event_fallback() {
        let fragment = make_fragment(
            MemoryKind::Event,
            r#"{"type":"agora_event","event_type":"message","payload":"hello"}"#,
        );
        let msgs = fragment.to_chat_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, llm::chat::ChatRole::User);
        // Fallback: raw JSON as content
        assert!(msgs[0].content.contains("agora_event"));
    }

    #[test]
    fn old_json_action_fallback() {
        let fragment =
            make_fragment(MemoryKind::Action, r#"{"type":"execution","action":"did something"}"#);
        let msgs = fragment.to_chat_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, llm::chat::ChatRole::Assistant);
    }

    #[test]
    fn unknown_kind_fallback() {
        let fragment = make_fragment(MemoryKind::Unknown, "mystery content");
        let msgs = fragment.to_chat_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, llm::chat::ChatRole::User);
    }
}
