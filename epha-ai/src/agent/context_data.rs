use epha_agent::context::ContextSerialize;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct PerceptionData {
    pub content: String,
    pub source: String,
    pub timestamp: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct MemoryData {
    pub content: String,
    pub keywords: Vec<String>,
    pub timestamp: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct ActivityRecord {
    pub action: String,
    pub timestamp: OffsetDateTime,
    pub details: String,
}

pub struct EphemeraContext {
    perception_buffer: Vec<PerceptionData>,
    memory_context: Vec<MemoryData>,
    activity_history: Vec<ActivityRecord>,
}

impl EphemeraContext {
    pub fn new() -> Self {
        Self {
            perception_buffer: Vec::new(),
            memory_context: Vec::new(),
            activity_history: Vec::new(),
        }
    }

    // Perception management
    pub fn add_perception(&mut self, content: String, source: String) {
        self.perception_buffer.push(PerceptionData {
            content,
            source,
            timestamp: OffsetDateTime::now_utc(),
        });
    }

    pub fn consume_perceptions(&mut self) -> Vec<PerceptionData> {
        let perceptions = self.perception_buffer.clone();
        self.perception_buffer.clear();

        if !perceptions.is_empty() {
            self.add_activity("perception".to_string(),
                format!("Perceived {} items from various sources", perceptions.len()));
        }

        perceptions
    }

    // Memory management
    pub fn update_memory_context(&mut self, memories: Vec<MemoryData>) {
        self.memory_context = memories;
        self.add_activity("memory_recall".to_string(),
            format!("Recalled {} memory fragments", self.memory_context.len()));
    }

    // Activity tracking
    pub fn add_activity(&mut self, action: String, details: String) {
        self.activity_history.push(ActivityRecord {
            action,
            timestamp: OffsetDateTime::now_utc(),
            details,
        });

        // Keep only last 20 activities
        if self.activity_history.len() > 20 {
            self.activity_history.remove(0);
        }
    }

    // Getters
    pub fn perception_buffer(&self) -> &[PerceptionData] {
        &self.perception_buffer
    }

    pub fn memory_context(&self) -> &[MemoryData] {
        &self.memory_context
    }

    pub fn activity_history(&self) -> &[ActivityRecord] {
        &self.activity_history
    }
}

impl ContextSerialize for EphemeraContext {
    fn serialize(&self) -> String {
        let mut output = String::new();

        // Memory context
        if !self.memory_context.is_empty() {
            output.push_str("Active Memory Context:\n");
            for memory in &self.memory_context {
                output.push_str(&format!("- {}\n", memory.content));
            }
            output.push_str("\n");
        }

        // Current perceptions
        if !self.perception_buffer.is_empty() {
            output.push_str("Current Perceptions:\n");
            for perception in &self.perception_buffer {
                output.push_str(&format!("- [{}] {}\n", perception.source, perception.content));
            }
            output.push_str("\n");
        }

        // Recent activity history
        if !self.activity_history.is_empty() {
            output.push_str("Recent Activity History:\n");
            for activity in self.activity_history.iter().rev().take(10) {
                if let Ok(formatted_time) = activity.timestamp.time().format(&time::format_description::parse("[hour]:[minute]:[second]").unwrap()) {
                    output.push_str(&format!("- [{}] {}: {}\n",
                        formatted_time,
                        activity.action,
                        activity.details));
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