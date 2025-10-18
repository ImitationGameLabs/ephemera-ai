use epha_agent::context::ContextSerialize;
use epha_memory::{MemoryFragment, MemorySource, SubjectiveMetadata, ObjectiveMetadata};
use time::OffsetDateTime;
use std::collections::VecDeque;
use std::fmt;

#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub activity_count: usize,
    pub current_token_usage: usize,
    pub max_token_limit: usize,
    pub utilization_ratio: f64,
}

impl fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Activities: {}, Tokens: {}/{} ({:.1}%)",
            self.activity_count,
            self.current_token_usage,
            self.max_token_limit,
            self.utilization_ratio * 100.0
        )
    }
}

pub struct EphemeraContext {
    memory_context: Vec<MemoryFragment>,           // Recalled long-term memories
    recent_activities: VecDeque<MemoryFragment>,   // Recent activities (including perceptions)
    current_token_usage: usize,                    // Current token usage
    max_token_limit: usize,                        // Maximum token limit
}

impl EphemeraContext {
    pub fn new() -> Self {
        Self {
            memory_context: Vec::new(),
            recent_activities: VecDeque::new(),
            current_token_usage: 0,
            max_token_limit: 30_000,  // 30k token maximum
        }
    }

    // Token estimation methods
    fn estimate_tokens(&self, text: &str) -> usize {
        // Temporary token estimation using byte count
        // Conservative estimate: 1 token ≈ 3 bytes
        text.len() / 3
    }

    fn calculate_fragment_tokens(&self, fragment: &MemoryFragment) -> usize {
        let mut tokens = self.estimate_tokens(&fragment.content);

        // Add tokens for tags
        for tag in &fragment.subjective_metadata.tags {
            tokens += self.estimate_tokens(tag);
        }

        // Add tokens for metadata
        tokens += self.estimate_tokens(&format!("{}", fragment.objective_metadata.source));
        tokens += self.estimate_tokens(&fragment.subjective_metadata.notes);

        // Add estimation for other fields
        tokens += 20; // Estimate for other structured fields

        tokens
    }

    // Perception as activity (unified approach)
    pub fn add_perception_activity(&mut self, content: String, source: String) {
        let perception_fragment = MemoryFragment {
            id: 0, // Temporary ID
            content,
            subjective_metadata: SubjectiveMetadata {
                importance: 120,
                confidence: 200,
                tags: vec!["perception".to_string(), source.clone()],
                notes: String::new(),
            },
            objective_metadata: ObjectiveMetadata {
                created_at: OffsetDateTime::now_utc().unix_timestamp(),
                source: MemorySource::dialogue_input(source),
            },
            associations: Vec::new(),
        };

        self.add_activity_fragment(perception_fragment);
    }

    // General activity method
    pub fn add_activity(&mut self, fragment: MemoryFragment) {
        self.add_activity_fragment(fragment);
    }

    // Memory management (now uses MemoryFragment)
    pub fn update_memory_context(&mut self, memories: Vec<MemoryFragment>) {
        self.memory_context = memories;
    }

    /// Add specific memory fragments to context with summary
    pub fn add_memory_context(&mut self, summary: String, memories: Vec<MemoryFragment>) {
        let memory_count = memories.len();

        // Add memories to context (avoiding duplicates)
        for memory in memories {
            if !self.memory_context.iter().any(|m| m.id == memory.id) {
                self.memory_context.push(memory);
            }
        }

        // Add activity entry with agent's summary
        let activity_fragment = MemoryFragmentBuilder::memory_selection(memory_count, summary).build();
        self.add_activity(activity_fragment);
    }

    // Unified activity fragment management
    fn add_activity_fragment(&mut self, fragment: MemoryFragment) {
        let fragment_tokens = self.calculate_fragment_tokens(&fragment);

        // Add to queue tail
        self.recent_activities.push_back(fragment);
        self.current_token_usage += fragment_tokens;

        // Maintain token limit
        self.maintain_token_limit();
    }

    fn maintain_token_limit(&mut self) {
        // Remove oldest activities if exceeding maximum limit
        while self.current_token_usage > self.max_token_limit && !self.recent_activities.is_empty() {
            if let Some(removed_fragment) = self.recent_activities.pop_front() {
                let removed_tokens = self.calculate_fragment_tokens(&removed_fragment);
                self.current_token_usage = self.current_token_usage.saturating_sub(removed_tokens);

                // Optional: log removed activity
                println!("Removed activity to maintain token limit: {} tokens", removed_tokens);
            }
        }
    }

    // Get current status information
    pub fn get_queue_status(&self) -> QueueStatus {
        QueueStatus {
            activity_count: self.recent_activities.len(),
            current_token_usage: self.current_token_usage,
            max_token_limit: self.max_token_limit,
            utilization_ratio: self.current_token_usage as f64 / self.max_token_limit as f64,
        }
    }

    // Token limit configuration
    pub fn set_token_limit(&mut self, max_limit: usize) {
        self.max_token_limit = max_limit;
        self.maintain_token_limit(); // Re-adjust queue
    }
}

impl ContextSerialize for EphemeraContext {
    fn serialize(&self) -> String {
        let mut output = String::new();

        // Memory context
        if !self.memory_context.is_empty() {
            output.push_str("Active Memory Context:\n");
            for memory in &self.memory_context {
                output.push_str(&format!("- [{}] {}\n",
                    memory.subjective_metadata.tags.join(", "),
                    memory.content));
            }
            output.push_str("\n");
        }

        // Recent activities with token usage info
        if !self.recent_activities.is_empty() {
            let status = self.get_queue_status();
            output.push_str(&format!("Recent Activities ({}):\n", status));

            for activity in self.recent_activities.iter() {
                let _activity_type = activity.subjective_metadata.tags
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("unknown");

                let tokens = self.calculate_fragment_tokens(activity);

                if let Ok(dt) = time::OffsetDateTime::from_unix_timestamp(activity.objective_metadata.created_at) {
                    if let Ok(formatted_time) = dt.time().format(&time::format_description::parse("[hour]:[minute]:[second]").unwrap()) {
                    output.push_str(&format!("- [{}] [{}] [{} tokens] {}\n",
                        formatted_time,
                        activity.objective_metadata.source,
                        tokens,
                        activity.content));
                    }
                }
            }
        }

        output
    }
}

impl Default for EphemeraContext {
    fn default() -> Self {
        Self::new()
    }
}

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

    // Convenience methods for common use cases

    /// Create an action activity
    pub fn action(action: String, details: String) -> Self {
        Self::new()
            .content(format!("{}: {}", action, details))
            .importance(100)
            .confidence(255)
            .add_tag("activity".to_string())
            .add_tag(action.clone())
            .source(MemorySource::action("execution".to_string()))
    }

    /// Create a thinking activity
    pub fn thinking(content: String) -> Self {
        Self::new()
            .content(content)
            .importance(120)
            .confidence(200)
            .add_tag("thinking".to_string())
            .source(MemorySource::thought("reasoning".to_string()))
    }

    /// Create a reflection activity
    pub fn reflection(content: String) -> Self {
        Self::new()
            .content(content)
            .importance(140)
            .confidence(180)
            .add_tag("reflection".to_string())
            .source(MemorySource::thought("meta_cognition".to_string()))
    }

    /// Create a perception activity
    pub fn perception(content: String, source: String) -> Self {
        Self::new()
            .content(content)
            .importance(120)
            .confidence(200)
            .add_tag("perception".to_string())
            .source(MemorySource::dialogue_input(source))
    }

    /// Create a memory selection activity
    pub fn memory_selection(count: usize, summary: String) -> Self {
        Self::new()
            .content(format!("Added {} memories to context. Summary: {}", count, summary))
            .importance(110)
            .confidence(255)
            .add_tag("memory_selection".to_string())
            .source(MemorySource::action("context_update".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use epha_memory::MemorySource;

    #[test]
    fn test_ephemera_context_new() {
        let context = EphemeraContext::new();

        assert_eq!(context.memory_context.len(), 0);
        assert_eq!(context.recent_activities.len(), 0);
        assert_eq!(context.get_queue_status().current_token_usage, 0);
        assert_eq!(context.get_queue_status().max_token_limit, 30_000);
    }

    #[test]
    fn test_token_estimation() {
        let context = EphemeraContext::new();

        let tokens = context.estimate_tokens("Hello world");
        assert_eq!(tokens, 11 / 3); // "Hello world" is 11 bytes, divided by 3

        let tokens = context.estimate_tokens("");
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_add_perception_activity() {
        let mut context = EphemeraContext::new();

        context.add_perception_activity(
            "Test perception".to_string(),
            "test_source".to_string()
        );

        assert_eq!(context.recent_activities.len(), 1);
        assert!(context.get_queue_status().current_token_usage > 0);

        let perception = context.recent_activities.back().unwrap();
        assert_eq!(perception.content, "Test perception");
        assert!(perception.subjective_metadata.tags.contains(&"perception".to_string()));
    }

    #[test]
    fn test_add_activity() {
        let mut context = EphemeraContext::new();

        let activity_fragment = MemoryFragmentBuilder::action(
            "test_action".to_string(),
            "test_details".to_string()
        ).build();
        context.add_activity(activity_fragment);

        assert_eq!(context.recent_activities.len(), 1);
        assert!(context.get_queue_status().current_token_usage > 0);

        let activity = context.recent_activities.back().unwrap();
        assert_eq!(activity.content, "test_action: test_details");
        assert!(activity.subjective_metadata.tags.contains(&"activity".to_string()));
    }

    #[test]
    fn test_token_limit_maintenance() {
        let mut context = EphemeraContext::new();

        // Set very low token limit for testing
        context.set_token_limit(200);

        // Add multiple activities to trigger token limit
        for i in 0..10 {
            let activity_fragment = MemoryFragmentBuilder::action(
                format!("action_{}", i),
                "A".repeat(100) // Large content to consume tokens
            ).build();
            context.add_activity(activity_fragment);
        }

        // Should maintain token limit by removing old activities
        assert!(context.get_queue_status().current_token_usage <= 200);
        assert!(context.recent_activities.len() < 10);
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

    #[test]
    fn test_queue_status_display() {
        let status = QueueStatus {
            activity_count: 5,
            current_token_usage: 15_000,
            max_token_limit: 30_000,
            utilization_ratio: 0.5,
        };

        assert_eq!(format!("{}", status), "Activities: 5, Tokens: 15000/30000 (50.0%)");
    }

    #[test]
    fn test_context_serialization() {
        let mut context = EphemeraContext::new();

        context.add_perception_activity(
            "Hello world".to_string(),
            "user".to_string()
        );

        let activity_fragment = MemoryFragmentBuilder::action(
            "greeting".to_string(),
            "Responded to user".to_string()
        ).build();
        context.add_activity(activity_fragment);

        let serialized = context.serialize();
        assert!(serialized.contains("Recent Activities"));
        assert!(serialized.contains("Hello world"));
        assert!(serialized.contains("greeting: Responded to user"));
    }
}