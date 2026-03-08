use super::MemoryFragmentList;
use super::memory_constructors::{from_action, from_agora_event};
use crate::sync::SyncSender;
use epha_agent::context::ContextSerialize;
use agora::event::Event;
use loom_client::memory::MemoryFragment;
use loom_client::{CreateMemoryRequest, LoomClient};
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::error;

// Re-export PinnedMemory for external use
pub use loom_client::PinnedMemory;

#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub activity_count: usize,
    pub current_token_usage: usize,
    pub max_token_limit: usize,
    pub utilization_ratio: f64,
}

impl fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Activities: {}, Tokens: {}/{} ({:.1}%)",
            self.activity_count,
            self.current_token_usage,
            self.max_token_limit,
            self.utilization_ratio * 100.0
        )
    }
}

pub struct EphemeraContext {
    loom_client: Arc<LoomClient>,
    sync_sender: SyncSender,
    pinned_memories: Vec<PinnedMemory>,
    memory_context: Vec<MemoryFragment>,
    recent_activities: VecDeque<MemoryFragment>,
    current_token_usage: usize,
    max_token_limit: usize,
    max_pinned_count: usize,
}

impl EphemeraContext {
    pub fn new(
        loom_client: Arc<LoomClient>,
        sync_sender: SyncSender,
        max_pinned_count: usize,
    ) -> Self {
        Self {
            pinned_memories: Vec::new(),
            memory_context: Vec::new(),
            recent_activities: VecDeque::new(),
            current_token_usage: 0,
            max_token_limit: 30_000,
            max_pinned_count,
            sync_sender,
            loom_client,
        }
    }

    /// Pin a memory by ID via Loom API
    /// This is an async operation that persists to the database
    pub async fn pin(&mut self, memory_id: i64, reason: String) -> Result<(), String> {
        if self.pinned_memories.len() >= self.max_pinned_count {
            return Err(format!(
                "Maximum pinned count ({}) reached, please unpin some content first",
                self.max_pinned_count
            ));
        }

        // Check if already pinned
        if self.pinned_memories.iter().any(|p| p.fragment.id == memory_id) {
            return Err(format!("Memory {} is already pinned", memory_id));
        }

        // Call Loom API to pin
        let pinned = self.loom_client.pin_memory(memory_id, Some(reason))
            .await
            .map_err(|e| format!("Failed to pin memory: {:?}", e))?;

        self.pinned_memories.push(pinned);
        self.recalculate_token_usage();
        Ok(())
    }

    /// Unpin a memory by ID via Loom API
    /// This is an async operation that persists to the database
    pub async fn unpin(&mut self, memory_id: i64) -> bool {
        // Call Loom API to unpin
        if self.loom_client.unpin_memory(memory_id).await.is_ok() {
            let before = self.pinned_memories.len();
            self.pinned_memories.retain(|p| p.fragment.id != memory_id);
            let removed = self.pinned_memories.len() < before;
            if removed {
                self.recalculate_token_usage();
            }
            return removed;
        }
        false
    }

    /// Get all pinned memories
    pub fn list_pinned(&self) -> &[PinnedMemory] {
        &self.pinned_memories
    }

    /// Get max pinned count
    pub fn max_pinned_count(&self) -> usize {
        self.max_pinned_count
    }

    /// Add a pinned memory to local state (called after successful API pin)
    pub fn add_pinned_memory(&mut self, pinned: PinnedMemory) {
        self.pinned_memories.push(pinned);
        self.recalculate_token_usage();
    }

    /// Remove a pinned memory from local state (called after successful API unpin)
    /// Returns true if the memory was found and removed
    pub fn remove_pinned_memory(&mut self, memory_id: i64) -> bool {
        let before = self.pinned_memories.len();
        self.pinned_memories.retain(|p| p.fragment.id != memory_id);
        let removed = self.pinned_memories.len() < before;
        if removed {
            self.recalculate_token_usage();
        }
        removed
    }

    /// Restore pinned memories from Loom on startup
    pub async fn restore_pinned_from_loom(&mut self) -> Result<(), String> {
        use tracing::info;

        info!("Restoring pinned memories from Loom");

        let response = self.loom_client.get_pinned_memories().await
            .map_err(|e| format!("Failed to fetch pinned memories: {:?}", e))?;

        if response.items.is_empty() {
            info!("No pinned memories found in Loom");
            return Ok(());
        }

        self.pinned_memories = response.items;
        self.recalculate_token_usage();

        info!("Restored {} pinned memories from Loom", self.pinned_memories.len());

        Ok(())
    }

    fn recalculate_token_usage(&mut self) {
        let mut total = 0;

        // Pinned memories tokens
        for item in &self.pinned_memories {
            total += self.estimate_tokens(&item.fragment.content);
        }

        // Memory context tokens
        for memory in &self.memory_context {
            total += self.calculate_fragment_tokens(memory);
        }

        // Recent activities tokens
        for activity in &self.recent_activities {
            total += self.calculate_fragment_tokens(activity);
        }

        self.current_token_usage = total;
    }

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

    pub fn add_activity(&mut self, fragment: MemoryFragment) {
        let fragment_tokens = self.calculate_fragment_tokens(&fragment);

        let mut temp_fragment = fragment.clone();
        temp_fragment.id = 0; // Temporary ID for tracking
        self.recent_activities.push_back(temp_fragment);
        self.current_token_usage += fragment_tokens;

        // Add to sync queue for background sync to Loom
        self.sync_sender.send(fragment);

        self.maintain_token_limit();
    }

    /// Add specific memory fragments to context with summary
    pub fn add_memory_context(&mut self, summary: String, memories: Vec<MemoryFragment>) {
        let memory_count = memories.len();

        for memory in memories {
            if !self.memory_context.iter().any(|m| m.id == memory.id) {
                self.memory_context.push(memory);
            }
        }

        let summary_fragment = from_action(
            format!(
                "Added {} memories to context. Summary: {}",
                memory_count, summary
            ),
            "context_update",
        )
        .build();

        self.add_activity(summary_fragment);
    }

    #[cfg(test)]
    pub fn add_memories_for_testing(&mut self, memories: Vec<MemoryFragment>) {
        for memory in memories {
            if !self.memory_context.iter().any(|m| m.id == memory.id) {
                self.memory_context.push(memory);
            }
        }
    }

    pub fn add_agora_events(&mut self, events: Vec<Event>) {
        for event in events {
            let fragment = from_agora_event(event).build();
            self.add_activity(fragment);
        }
    }

    fn maintain_token_limit(&mut self) {
        while self.current_token_usage > self.max_token_limit && !self.recent_activities.is_empty()
        {
            if let Some(removed_fragment) = self.recent_activities.pop_front() {
                let removed_tokens = self.calculate_fragment_tokens(&removed_fragment);
                self.current_token_usage = self.current_token_usage.saturating_sub(removed_tokens);
            }
        }
    }

    pub fn get_queue_status(&self) -> QueueStatus {
        QueueStatus {
            activity_count: self.recent_activities.len(),
            current_token_usage: self.current_token_usage,
            max_token_limit: self.max_token_limit,
            utilization_ratio: self.current_token_usage as f64 / self.max_token_limit as f64,
        }
    }

    pub fn set_token_limit(&mut self, max_limit: usize) {
        self.max_token_limit = max_limit;
        self.maintain_token_limit();
    }

    /// Restore recent activities from Loom on startup
    /// This is called after context creation to recover state after a crash
    pub async fn restore_from_loom(&mut self, limit: usize) -> Result<(), String> {
        use tracing::info;

        info!("Restoring recent activities from Loom (limit: {})", limit);

        let response = self.loom_client.get_recent_memories(limit).await
            .map_err(|e| format!("Failed to fetch recent memories: {:?}", e))?;

        if response.fragments.is_empty() {
            info!("No recent memories found in Loom");
            return Ok(());
        }

        // Add fragments directly to recent_activities without going through sync_sender.
        // This avoids a sync loop: we're restoring FROM Loom, so there's no need to sync back TO Loom.
        for fragment in response.fragments {
            let fragment_tokens = self.calculate_fragment_tokens(&fragment);
            self.recent_activities.push_back(fragment);
            self.current_token_usage += fragment_tokens;
        }

        info!("Restored {} memories from Loom", self.recent_activities.len());

        // Maintain token limit after restoration
        self.maintain_token_limit();

        Ok(())
    }
}

impl ContextSerialize for EphemeraContext {
    fn serialize(&self) -> String {
        let mut output = String::new();

        // 1. Pinned content (highest priority)
        if !self.pinned_memories.is_empty() {
            output.push_str("📌 Pinned Content:\n");
            output.push_str("---\n");
            for item in &self.pinned_memories {
                let reason_str = item.reason.as_deref().unwrap_or("N/A");
                output.push_str(&format!(
                    "[id:{}] {} (Reason: {})\n",
                    item.fragment.id, item.fragment.content, reason_str
                ));
            }
            output.push_str("---\n");
        }

        // 2. Active memory context
        if !self.memory_context.is_empty() {
            output.push_str("Active Memory Context:\n");
            output.push_str("---\n");
            let serialized_memories =
                MemoryFragmentList::from(self.memory_context.clone()).serialize();
            output.push_str(&serialized_memories);
            output.push_str("---\n");
        }

        // 3. Recent activities
        if !self.recent_activities.is_empty() {
            let status = self.get_queue_status();
            output.push_str(&format!("Recent Activities ({}):\n", status));
            output.push_str("---\n");
            let serialized_activities =
                MemoryFragmentList::from(self.recent_activities.clone()).serialize();
            output.push_str(&serialized_activities);
            output.push_str("---\n");
        }

        output
    }
}
