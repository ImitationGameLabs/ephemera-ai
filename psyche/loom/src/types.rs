use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFragment {
    pub id: i64,
    pub content: String,

    pub subjective_metadata: SubjectiveMetadata,
    pub objective_metadata: ObjectiveMetadata,

    pub associations: Vec<i64>,
}

/// ObjectiveMetadata representing the system's definitive record of a memory fragment.
/// Contains objective facts about the memory that are autonomously maintained by the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveMetadata {
    /// Timestamp when the memory was created.
    pub created_at: i64,
    /// Source of the memory, indicating its origin (user input, system thought, etc.).
    pub source: MemorySource,
}

/// SubjectiveMetadata representing the AI system's subjective perception of a memory fragment
/// Contains the AI's subjective evaluation of memory importance, confidence, etc.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubjectiveMetadata {
    /// Importance assessment of the memory, range 0-255, higher values indicate greater importance.
    pub importance: u8,
    /// Confidence assessment of the memory, range 0-255, higher values indicate stronger AI confidence.
    pub confidence: u8,
    /// Tags associated with the memory for categorization and retrieval.
    pub tags: Vec<String>,
    /// Free-form notes.
    pub notes: String, // TODO: Implement emotional weights for sentiment tracking
                       // pub emotional_weight: EmotionalWeights,
}

/// Represents the origin of a memory fragment with channel-based design.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySource {
    /// Channel category: "dialogue", "information", "thought", "action"
    pub channel: String,
    /// Unique identifier for the specific source instance
    pub identifier: String,
    /// Additional metadata for rich source information
    pub metadata: HashMap<String, String>,
}

impl Default for MemorySource {
    fn default() -> Self {
        Self {
            channel: "unknown".to_string(),
            identifier: "unknown".to_string(),
            metadata: HashMap::new(),
        }
    }
}

impl fmt::Display for MemorySource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let type_str = self.metadata.get("type")
            .map(|t| format!(":{}", t))
            .unwrap_or_default();
        write!(f, "[{}{}] {}", self.channel, type_str, self.identifier)
    }
}

// Convenience methods for creating common MemorySource types
impl MemorySource {
    pub fn dialogue_input(identifier: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "input".to_string());

        Self {
            channel: "dialogue".to_string(),
            identifier,
            metadata,
        }
    }

    pub fn dialogue_response() -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "output".to_string());

        Self {
            channel: "dialogue".to_string(),
            identifier: "self".to_string(),
            metadata,
        }
    }

    pub fn information(identifier: String, info_type: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), info_type);

        Self {
            channel: "information".to_string(),
            identifier,
            metadata,
        }
    }

    pub fn thought(thought_type: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), thought_type);

        Self {
            channel: "thought".to_string(),
            identifier: "self_thought".to_string(),
            metadata,
        }
    }

    pub fn action(action_type: String) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), action_type);

        Self {
            channel: "action".to_string(),
            identifier: "self_action".to_string(),
            metadata,
        }
    }
}
