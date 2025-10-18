use super::{ContextSerialize, MemoryFragment, MemoryFragmentList};
use epha_memory::{MemorySource, MemoryFragmentBuilder, HybridMemoryManager, Manager};
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use tracing::error;

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
    memory_manager: Arc<HybridMemoryManager>,      // Memory manager for auto-saving

    memory_context: Vec<MemoryFragment>,           // Recalled long-term memories
    recent_activities: VecDeque<MemoryFragment>,   // Recent activities

    current_token_usage: usize,                    // Current token usage
    max_token_limit: usize,                        // Maximum token limit
}

impl EphemeraContext {
    pub fn new(memory_manager: Arc<HybridMemoryManager>) -> Self {
        Self {
            memory_context: Vec::new(),
            recent_activities: VecDeque::new(),
            current_token_usage: 0,
            max_token_limit: 30_000,  // 30k token maximum
            memory_manager,
        }
    }

    
    // Token estimation methods
    fn estimate_tokens(&self, text: &str) -> usize {
        // Token estimation using character count
        // Rough estimate: 1 token ≈ 4 characters (more accurate than byte count for UTF-8)
        text.chars().count() / 4
    }

    fn calculate_fragment_tokens(&self, fragment: &MemoryFragment) -> usize {
        // Use actual serialized content for accurate token calculation
        // This ensures token count matches what will actually be sent to AI
        let memory_list = super::MemoryFragmentList::from(vec![fragment.clone()]);
        let serialized = memory_list.serialize();
        self.estimate_tokens(&serialized)
    }

    // General activity method - single interface for adding activities
    pub fn add_activity(&mut self, fragment: MemoryFragment) {
        let fragment_tokens = self.calculate_fragment_tokens(&fragment);

        // Add to queue tail
        self.recent_activities.push_back(fragment.clone());
        self.current_token_usage += fragment_tokens;

        // Auto-save to long-term memory (async-friendly approach)
        let fragment_to_save = fragment.clone();
        let memory_manager = self.memory_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = memory_manager.append(&fragment_to_save).await {
                error!("Failed to save activity to memory: {:?}", e);
            }
        });

        // Maintain token limit
        self.maintain_token_limit();
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
        let activity_fragment = MemoryFragmentBuilder::new()
            .content(format!("Added {} memories to context. Summary: {}", memory_count, summary))
            .importance(110)
            .confidence(255)
            .add_tag("memory_selection".to_string())
            .source(MemorySource::action("context_update".to_string()))
            .build();
        self.add_activity(activity_fragment);
    }


    fn maintain_token_limit(&mut self) {
        // Remove oldest activities if exceeding maximum limit
        while self.current_token_usage > self.max_token_limit && !self.recent_activities.is_empty() {
            if let Some(removed_fragment) = self.recent_activities.pop_front() {
                let removed_tokens = self.calculate_fragment_tokens(&removed_fragment);
                self.current_token_usage = self.current_token_usage.saturating_sub(removed_tokens);
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
            let serialized_memories = MemoryFragmentList::from(self.memory_context.clone()).serialize();
            output.push_str(&serialized_memories);
            output.push_str("\n");
        }

        // Recent activities
        if !self.recent_activities.is_empty() {
            let status = self.get_queue_status();
            output.push_str(&format!("Recent Activities ({}):\n", status));

            let serialized_activities = MemoryFragmentList::from(self.recent_activities.clone()).serialize();
            output.push_str(&serialized_activities);
        }

        output
    }
}

// Default implementation removed since EphemeraContext now requires a memory_manager

#[cfg(test)]
mod tests {
    use super::*;

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
}