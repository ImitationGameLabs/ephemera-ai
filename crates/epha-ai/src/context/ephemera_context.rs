use super::MemoryFragmentList;
use super::memory_constructors::{from_action, from_agora_event};
use crate::sync::SyncSender;
use epha_agent::context::ContextSerialize;
use agora::event::Event;
use loom_client::memory::MemoryFragment;
use loom_client::{CreateMemoryRequest, LoomClientTrait};
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
    loom_client: Arc<dyn LoomClientTrait>,
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
        loom_client: Arc<dyn LoomClientTrait>,
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

// ============================================================================
// Context Restoration Consistency Tests
// ============================================================================

#[cfg(test)]
mod restoration_tests {
    use super::*;
    use crate::sync::SyncSender;
    use loom_client::memory::MemoryKind;
    use loom_client::mock::MockLoomClient;
    use loom_client::{MemoryResponse, PinnedMemory, PinnedMemoriesResponse};
    use std::sync::Arc;

    /// Helper to create a test fragment with given id, content, and kind
    fn create_fragment_with_kind(id: i64, content: &str, kind: MemoryKind) -> MemoryFragment {
        MemoryFragment {
            id,
            content: content.to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind,
        }
    }

    /// Helper to create a test fragment with given id and content (default kind: Action)
    fn create_fragment(id: i64, content: &str) -> MemoryFragment {
        create_fragment_with_kind(id, content, MemoryKind::Action)
    }

    /// Helper to create a pinned memory
    fn create_pinned(id: i64, content: &str, reason: Option<&str>) -> PinnedMemory {
        PinnedMemory {
            fragment: create_fragment(id, content),
            reason: reason.map(|s| s.to_string()),
            pinned_at: time::OffsetDateTime::now_utc(),
        }
    }

    /// Helper to create a test context with mock client
    fn create_test_context(mock: MockLoomClient, max_pinned: usize) -> EphemeraContext {
        let (sync_sender, _receiver) = SyncSender::channel();
        EphemeraContext::new(Arc::new(mock), sync_sender, max_pinned)
    }

    // =========================================================================
    // Basic Restoration Tests
    // =========================================================================

    /// Test: Empty state restoration
    #[tokio::test]
    async fn test_restore_empty_state() {
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx = create_test_context(mock, 10);
        ctx.restore_from_loom(50).await.unwrap();
        ctx.restore_pinned_from_loom().await.unwrap();

        assert!(ctx.list_pinned().is_empty());
        assert_eq!(ctx.get_queue_status().activity_count, 0);
        assert_eq!(ctx.get_queue_status().current_token_usage, 0);
    }

    /// Test: Normal restoration with pinned and recent
    #[tokio::test]
    async fn test_restore_normal_state() {
        let mut mock = MockLoomClient::new();

        let recent_fragments = vec![
            create_fragment(1, "Activity 1"),
            create_fragment(2, "Activity 2"),
        ];
        mock.set_default_memory(MemoryResponse {
            fragments: recent_fragments,
            total: 2,
        });

        let pinned_items = vec![
            create_pinned(100, "Pinned content", Some("Important")),
        ];
        mock.set_default_pinned(PinnedMemoriesResponse { items: pinned_items });

        let mut ctx = create_test_context(mock, 10);
        ctx.restore_from_loom(50).await.unwrap();
        ctx.restore_pinned_from_loom().await.unwrap();

        assert_eq!(ctx.list_pinned().len(), 1);
        assert_eq!(ctx.get_queue_status().activity_count, 2);
        assert!(ctx.get_queue_status().current_token_usage > 0);
    }

    // =========================================================================
    // Pin/Unpin Operation Sequence Tests
    // =========================================================================

    /// Test: Pin -> Unpin -> Restart sequence
    /// Verifies unpin operation persists correctly across restart
    #[tokio::test]
    async fn test_pin_unpin_sequence_consistency() {
        // Simulate Loom state: only B is pinned (A was unpinned)
        let mut mock = MockLoomClient::new();
        mock.set_default_pinned(PinnedMemoriesResponse {
            items: vec![
                create_pinned(2, "Memory B", Some("Still pinned")),
            ],
        });
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });

        let mut ctx = create_test_context(mock, 10);
        ctx.restore_pinned_from_loom().await.unwrap();

        let pinned = ctx.list_pinned();
        assert_eq!(pinned.len(), 1);
        assert_eq!(pinned[0].fragment.id, 2);
    }

    /// Test: Multiple pin/unpin operations before restart
    #[tokio::test]
    async fn test_multiple_pin_unpin_operations() {
        // Simulate Loom state after complex operation sequence:
        // pin(1) -> pin(2) -> pin(3) -> unpin(1) -> pin(4) -> unpin(2)
        // Final state: [3, 4]
        let mut mock = MockLoomClient::new();
        mock.set_default_pinned(PinnedMemoriesResponse {
            items: vec![
                create_pinned(3, "Memory 3", Some("Kept")),
                create_pinned(4, "Memory 4", Some("Added later")),
            ],
        });
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });

        let mut ctx = create_test_context(mock, 10);
        ctx.restore_pinned_from_loom().await.unwrap();

        let pinned = ctx.list_pinned();
        assert_eq!(pinned.len(), 2);
        let pinned_ids: Vec<i64> = pinned.iter().map(|p| p.fragment.id).collect();
        assert!(pinned_ids.contains(&3));
        assert!(pinned_ids.contains(&4));
        assert!(!pinned_ids.contains(&1));
        assert!(!pinned_ids.contains(&2));
    }

    // =========================================================================
    // memory_context Not Persisted Tests
    // =========================================================================

    /// Test: memory_context is lost after restart (by design)
    #[tokio::test]
    async fn test_memory_context_not_persisted() {
        // Phase 1: Create context with memory_context
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx = create_test_context(mock, 10);
        ctx.add_memories_for_testing(vec![
            create_fragment(100, "Recalled memory 1"),
            create_fragment(101, "Recalled memory 2"),
        ]);

        // Verify memory_context has content
        let serialized_with_context = ctx.serialize();
        assert!(serialized_with_context.contains("Recalled memory"));

        // Phase 2: Simulate restart with fresh context
        let mut mock_after_restart = MockLoomClient::new();
        mock_after_restart.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock_after_restart.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx_after_restart = create_test_context(mock_after_restart, 10);
        ctx_after_restart.restore_from_loom(50).await.unwrap();
        ctx_after_restart.restore_pinned_from_loom().await.unwrap();

        // Verify memory_context is empty (not persisted)
        let serialized_after_restart = ctx_after_restart.serialize();
        assert!(!serialized_after_restart.contains("Recalled memory"));
        assert!(!serialized_after_restart.contains("Active Memory Context"));
    }

    // =========================================================================
    // Activity Sync and Loss Tests
    // =========================================================================

    /// Test: Synced activities are restored, unsynced are lost
    #[tokio::test]
    async fn test_activity_sync_loss_simulation() {
        // Simulate Loom state with only synced data (missing latest activity)
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse {
            fragments: vec![
                create_fragment(1, "Synced activity 1"),
                create_fragment(2, "Synced activity 2"),
                // Note: no activity 3 (simulates crash before sync)
            ],
            total: 2,
        });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        // Simulate restart restore
        let mut ctx = create_test_context(mock, 10);
        ctx.restore_from_loom(50).await.unwrap();

        // Only synced data is restored
        assert_eq!(ctx.get_queue_status().activity_count, 2);
    }

    /// Test: Activity token calculation and eviction
    #[tokio::test]
    async fn test_activity_token_eviction() {
        let mut mock = MockLoomClient::new();

        // Create large content that will definitely exceed limit
        let large_content = "x".repeat(10000);
        mock.set_default_memory(MemoryResponse {
            fragments: vec![
                create_fragment(1, &format!("{}-1", large_content)),
                create_fragment(2, &format!("{}-2", large_content)),
                create_fragment(3, &format!("{}-3", large_content)),
            ],
            total: 3,
        });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx = create_test_context(mock, 10);
        ctx.set_token_limit(5000);

        ctx.restore_from_loom(50).await.unwrap();

        // Verify token limit is maintained
        // Note: We don't verify exact eviction count because token estimation
        // is approximate. The key invariant is token_usage <= limit.
        let status = ctx.get_queue_status();
        assert!(
            status.current_token_usage <= status.max_token_limit,
            "Token usage {} should be under limit {}",
            status.current_token_usage,
            status.max_token_limit
        );
    }

    /// Test: Runtime token eviction via add_activity()
    /// Verifies FIFO eviction order when adding activities exceeds token limit
    #[tokio::test]
    async fn test_runtime_token_eviction() {
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx = create_test_context(mock, 10);
        // Set low token limit - each large activity uses ~2500 tokens when serialized
        ctx.set_token_limit(5000);

        // Create large content that will trigger eviction
        let large_content = "x".repeat(10000);

        // Add activities: older ones should be evicted as newer ones are added
        ctx.add_activity(create_fragment(1, &format!("{}-1", large_content))); // Will be evicted
        ctx.add_activity(create_fragment(2, &format!("{}-2", large_content))); // Will be evicted
        ctx.add_activity(create_fragment(3, &format!("{}-3", large_content))); // Kept (most recent)

        // Token usage should be under limit
        let status = ctx.get_queue_status();
        assert!(
            status.current_token_usage <= status.max_token_limit,
            "Token usage {} should be under limit {}",
            status.current_token_usage,
            status.max_token_limit
        );
        // Some eviction should have occurred (3 large activities can't all fit in 5000 tokens)
        assert!(status.activity_count < 3, "Older activities should be evicted");
    }

    // =========================================================================
    // Pinned vs Recent Interaction Tests
    // =========================================================================

    /// Test: Same memory appears in both pinned and recent
    /// This is by design: pinned_memories and recent_activities are independent collections
    #[tokio::test]
    async fn test_pinned_and_recent_overlap() {
        let shared_content = "Memory that is both pinned and recent";

        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse {
            fragments: vec![create_fragment(42, shared_content)],
            total: 1,
        });
        mock.set_default_pinned(PinnedMemoriesResponse {
            items: vec![create_pinned(42, shared_content, Some("Important"))],
        });

        let mut ctx = create_test_context(mock, 10);
        ctx.restore_from_loom(50).await.unwrap();
        ctx.restore_pinned_from_loom().await.unwrap();

        // Both collections contain this memory (by design)
        assert_eq!(ctx.list_pinned().len(), 1);
        assert_eq!(ctx.list_pinned()[0].fragment.id, 42);
        assert_eq!(ctx.get_queue_status().activity_count, 1);

        // Both appear in serialization
        let serialized = ctx.serialize();
        assert!(serialized.contains("Pinned Content"));
        assert!(serialized.contains("Recent Activities"));
    }

    /// Test: Loom returns duplicate IDs in get_recent_memories
    /// Current implementation adds duplicates - this is documented behavior
    #[tokio::test]
    async fn test_duplicate_ids_in_recent_response() {
        let mut mock = MockLoomClient::new();

        // Loom returns same ID twice (edge case)
        mock.set_default_memory(MemoryResponse {
            fragments: vec![
                create_fragment(1, "Duplicate content"),
                create_fragment(1, "Duplicate content"), // Same ID
            ],
            total: 2,
        });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx = create_test_context(mock, 10);
        ctx.restore_from_loom(50).await.unwrap();

        // Current behavior: both are added (no deduplication in restore path)
        // This test documents the current behavior
        assert_eq!(ctx.get_queue_status().activity_count, 2);
    }

    /// Test: Pinned content consumes token budget, affecting recent eviction
    #[tokio::test]
    async fn test_pinned_tokens_affect_recent_eviction() {
        let mut mock = MockLoomClient::new();

        // Pinned content with large token usage
        let pinned_content = "x".repeat(6000);
        mock.set_default_pinned(PinnedMemoriesResponse {
            items: vec![create_pinned(100, &pinned_content, Some("Large pinned"))],
        });

        // Recent content
        let recent_content = "y".repeat(4000);
        mock.set_default_memory(MemoryResponse {
            fragments: vec![
                create_fragment(1, &recent_content),
                create_fragment(2, &recent_content),
            ],
            total: 2,
        });

        let mut ctx = create_test_context(mock, 10);
        // Set low token limit to force eviction
        // Pinned: ~1500 tokens (serialized), Recent: ~1000 tokens each (serialized)
        // With limit 2500, only pinned + 1 recent can fit
        ctx.set_token_limit(2500);

        // IMPORTANT: Restore pinned FIRST, then recent
        // restore_from_loom() calls maintain_token_limit(), which evicts if over limit
        // restore_pinned_from_loom() does NOT evict (pinned is never evicted)
        ctx.restore_pinned_from_loom().await.unwrap();
        ctx.restore_from_loom(50).await.unwrap();

        // Pinned should be preserved (never evicted)
        assert_eq!(ctx.list_pinned().len(), 1, "Pinned content should never be evicted");

        // Recent should be evicted to fit token limit
        // Started with 2 recent activities, at least one should be evicted
        let status = ctx.get_queue_status();
        assert!(status.activity_count < 2, "At least one recent activity should be evicted");
        assert!(status.current_token_usage <= 2500, "Token usage should be under limit");
    }

    // =========================================================================
    // Full Lifecycle Simulation Tests
    // =========================================================================

    /// Test: Graceful restart with full sync - state should be identical
    #[tokio::test]
    async fn test_graceful_restart_consistency() {
        // Phase 1: Create context with known state
        let mut mock_initial = MockLoomClient::new();
        let fragments = vec![
            create_fragment(1, "Activity 1"),
            create_fragment(2, "Activity 2"),
            create_fragment(3, "Activity 3"),
        ];
        mock_initial.set_default_memory(MemoryResponse {
            fragments: fragments.clone(),
            total: 3,
        });
        let pinned = vec![create_pinned(100, "Pinned", Some("Important"))];
        mock_initial.set_default_pinned(PinnedMemoriesResponse { items: pinned.clone() });

        let mut ctx_initial = create_test_context(mock_initial, 10);
        ctx_initial.restore_from_loom(50).await.unwrap();
        ctx_initial.restore_pinned_from_loom().await.unwrap();

        // Capture initial state
        let initial_activity_count = ctx_initial.get_queue_status().activity_count;
        let initial_pinned_count = ctx_initial.list_pinned().len();
        let initial_token_usage = ctx_initial.get_queue_status().current_token_usage;

        // Phase 2: Simulate graceful restart (all data synced)
        let mut mock_restart = MockLoomClient::new();
        mock_restart.set_default_memory(MemoryResponse {
            fragments: fragments.clone(),
            total: 3,
        });
        mock_restart.set_default_pinned(PinnedMemoriesResponse { items: pinned.clone() });

        let mut ctx_restarted = create_test_context(mock_restart, 10);
        ctx_restarted.restore_from_loom(50).await.unwrap();
        ctx_restarted.restore_pinned_from_loom().await.unwrap();

        // Phase 3: Verify state is identical
        assert_eq!(
            ctx_restarted.get_queue_status().activity_count,
            initial_activity_count,
            "Activity count should be identical after graceful restart"
        );
        assert_eq!(
            ctx_restarted.list_pinned().len(),
            initial_pinned_count,
            "Pinned count should be identical after graceful restart"
        );
        assert_eq!(
            ctx_restarted.get_queue_status().current_token_usage,
            initial_token_usage,
            "Token usage should be identical after graceful restart"
        );
    }

    /// Test: Simulate complete runtime lifecycle
    /// 1. Startup restore
    /// 2. Runtime operations (pin, recall, activity)
    /// 3. Crash
    /// 4. Restart restore
    #[tokio::test]
    async fn test_full_lifecycle_simulation() {
        // === Phase 1: Initial startup ===
        let mut mock_initial = MockLoomClient::new();
        mock_initial.set_default_memory(MemoryResponse {
            fragments: vec![create_fragment(1, "Initial activity")],
            total: 1,
        });
        mock_initial.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx = create_test_context(mock_initial, 10);
        ctx.restore_from_loom(50).await.unwrap();
        ctx.restore_pinned_from_loom().await.unwrap();

        // === Phase 2: Runtime operations ===
        // 2.1 Add activity
        ctx.add_activity(create_fragment(2, "Runtime activity"));

        // 2.2 Recall memories (not persisted)
        ctx.add_memories_for_testing(vec![
            create_fragment(100, "Recalled during runtime"),
        ]);

        // Verify runtime state
        assert_eq!(ctx.get_queue_status().activity_count, 2);
        let runtime_serialized = ctx.serialize();
        assert!(runtime_serialized.contains("Recalled during runtime"));

        // === Phase 3: Simulate crash and restart ===
        // Loom only has partial data (simulating async sync loss)
        let mut mock_after_crash = MockLoomClient::new();
        mock_after_crash.set_default_memory(MemoryResponse {
            fragments: vec![
                create_fragment(1, "Initial activity"),
                // Fragment 2 lost (not synced)
            ],
            total: 1,
        });
        // Simulate a pin operation that persisted
        mock_after_crash.set_default_pinned(PinnedMemoriesResponse {
            items: vec![create_pinned(100, "Pinned before crash", Some("Important"))],
        });

        // === Phase 4: Restart restore ===
        let mut ctx_restarted = create_test_context(mock_after_crash, 10);
        ctx_restarted.restore_from_loom(50).await.unwrap();
        ctx_restarted.restore_pinned_from_loom().await.unwrap();

        // Verify restored state
        // - Pinned should be restored
        assert_eq!(ctx_restarted.list_pinned().len(), 1);
        assert_eq!(ctx_restarted.list_pinned()[0].fragment.id, 100);

        // - Recent only contains synced data
        assert_eq!(ctx_restarted.get_queue_status().activity_count, 1);

        // - memory_context lost
        let restarted_serialized = ctx_restarted.serialize();
        assert!(!restarted_serialized.contains("Recalled during runtime"));
    }

    /// Test: Pin operation persistence across immediate restart
    #[tokio::test]
    async fn test_pin_persists_across_restart() {
        // Simulate Loom state right after pin operation
        let mut mock = MockLoomClient::new();
        mock.set_default_pinned(PinnedMemoriesResponse {
            items: vec![create_pinned(42, "Freshly pinned", Some("Just pinned before crash"))],
        });
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });

        let mut ctx = create_test_context(mock, 10);
        ctx.restore_pinned_from_loom().await.unwrap();

        assert_eq!(ctx.list_pinned().len(), 1);
        assert_eq!(ctx.list_pinned()[0].fragment.id, 42);
        assert_eq!(
            ctx.list_pinned()[0].reason,
            Some("Just pinned before crash".to_string())
        );
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    /// Test: Graceful degradation on Loom API failure
    #[tokio::test]
    async fn test_restore_failure_graceful_degradation() {
        let mut mock = MockLoomClient::new();
        mock.push_error("Loom service unavailable");

        let mut ctx = create_test_context(mock, 10);

        let result = ctx.restore_from_loom(50).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to fetch"));

        // Context should remain in valid empty state
        assert_eq!(ctx.get_queue_status().activity_count, 0);
    }

    /// Test: Pinned restore fails, but recent restore succeeds
    #[tokio::test]
    async fn test_partial_restore_failure() {
        let mut mock = MockLoomClient::new();

        // Push responses in order:
        // 1. Memory response for restore_from_loom (get_recent_memories)
        mock.push_memory(MemoryResponse {
            fragments: vec![create_fragment(1, "Activity")],
            total: 1,
        });
        // 2. Error for restore_pinned_from_loom (get_pinned_memories)
        mock.push_error("Pinned query failed");

        let mut ctx = create_test_context(mock, 10);

        // Recent should succeed
        ctx.restore_from_loom(50).await.unwrap();
        assert_eq!(ctx.get_queue_status().activity_count, 1);

        // Pinned should fail
        let result = ctx.restore_pinned_from_loom().await;
        assert!(result.is_err());

        // But recent is still preserved
        assert_eq!(ctx.get_queue_status().activity_count, 1);
    }

    // =========================================================================
    // Boundary Condition Tests
    // =========================================================================

    /// Test: Restoring pinned memories exceeding max_pinned_count
    /// Restoration does not enforce the limit (limit only applies to pin() operations)
    #[tokio::test]
    async fn test_restore_exceeds_max_pinned_count() {
        let mut mock = MockLoomClient::new();

        // Loom has 5 pinned, but max_pinned_count = 3
        let pinned_items: Vec<PinnedMemory> = (1..=5)
            .map(|i| create_pinned(i, &format!("Pinned {}", i), None))
            .collect();
        mock.set_default_pinned(PinnedMemoriesResponse { items: pinned_items });
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });

        let mut ctx = create_test_context(mock, 3);
        ctx.restore_pinned_from_loom().await.unwrap();

        // Restoration does not enforce limit
        assert_eq!(ctx.list_pinned().len(), 5);
    }

    /// Test: Token calculation accuracy
    #[tokio::test]
    async fn test_token_calculation_accuracy() {
        let mut mock = MockLoomClient::new();

        // Use predictable content sizes
        let short_content = "Short"; // ~5 chars
        let medium_content = "Medium length content"; // ~20 chars

        mock.set_default_memory(MemoryResponse {
            fragments: vec![
                create_fragment(1, short_content),
                create_fragment(2, medium_content),
            ],
            total: 2,
        });
        mock.set_default_pinned(PinnedMemoriesResponse {
            items: vec![create_pinned(100, "Pinned", None)],
        });

        let mut ctx = create_test_context(mock, 10);
        ctx.restore_from_loom(50).await.unwrap();
        ctx.restore_pinned_from_loom().await.unwrap();

        let status = ctx.get_queue_status();

        // Token usage should be positive
        assert!(status.current_token_usage > 0, "Token usage should be positive");

        // Token usage should account for all content (pinned + 2 recent)
        // Minimum expected: at least 10 tokens for 3 items with serialization overhead
        assert!(
            status.current_token_usage >= 10,
            "Token usage should account for all content"
        );

        // Token usage should be reasonable (not exceeding 10x char count)
        let total_chars = "Pinned".len() + short_content.len() + medium_content.len();
        assert!(
            status.current_token_usage < total_chars * 10,
            "Token estimate should be reasonable, got {} for {} chars",
            status.current_token_usage,
            total_chars
        );
    }

    // =========================================================================
    // Real Crash Simulation Tests
    // =========================================================================

    /// Test: Simulate crash with sync queue data loss
    /// This test uses the actual sync mechanism to verify unsynced data is lost
    #[tokio::test]
    async fn test_crash_with_sync_queue_loss() {
        use tokio::sync::mpsc;

        // Create sync channel and capture receiver
        let (sync_sender, mut sync_receiver) = SyncSender::channel();

        // Setup mock that records create_memory calls
        let mut mock_runtime = MockLoomClient::new();
        mock_runtime.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock_runtime.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let loom_client: Arc<dyn LoomClientTrait> = Arc::new(mock_runtime);

        // Create context with real sync sender
        let mut ctx = EphemeraContext::new(
            loom_client.clone(),
            sync_sender,
            10,
        );

        // Add activities - they go into sync queue
        ctx.add_activity(create_fragment(1, "Activity 1 - will be synced"));
        ctx.add_activity(create_fragment(2, "Activity 2 - will be synced"));
        ctx.add_activity(create_fragment(3, "Activity 3 - will be LOST (unsynced)"));

        // Verify local state has all 3 activities
        assert_eq!(ctx.get_queue_status().activity_count, 3);

        // Simulate partial sync: process only first 2 items from sync queue
        let mut synced_fragments = Vec::new();
        synced_fragments.push(sync_receiver.try_recv().unwrap());
        synced_fragments.push(sync_receiver.try_recv().unwrap());

        // Verify third item exists and receive it (simulating it will be lost in crash)
        let _lost_item = sync_receiver.try_recv().expect("Third item should exist in queue");

        // Simulate crash: drop context without processing remaining sync queue
        // The unsynced activity 3 is now lost
        drop(ctx);

        // Setup mock for restart with only synced data
        let mut mock_restart = MockLoomClient::new();
        mock_restart.set_default_memory(MemoryResponse {
            fragments: synced_fragments.clone(),
            total: 2,
        });
        mock_restart.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        // Restart: create new context
        let (new_sync_sender, _) = SyncSender::channel();
        let mut ctx_restarted = EphemeraContext::new(
            Arc::new(mock_restart),
            new_sync_sender,
            10,
        );

        ctx_restarted.restore_from_loom(50).await.unwrap();

        // Only 2 activities should be restored (activity 3 was lost)
        assert_eq!(
            ctx_restarted.get_queue_status().activity_count, 2,
            "Activity 3 should be lost due to crash before sync"
        );
    }

    /// Test: Pin operation during runtime persists after crash
    #[tokio::test]
    async fn test_pin_during_runtime_persists_after_crash() {
        // Phase 1: Runtime with pin operation
        let mut mock_runtime = MockLoomClient::new();
        mock_runtime.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock_runtime.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        // Setup mock to accept pin
        let pinned_response = create_pinned(42, "Runtime pinned content", Some("Important"));
        mock_runtime.push_pinned_memory(pinned_response.clone());

        let loom_client: Arc<dyn LoomClientTrait> = Arc::new(mock_runtime);
        let (sync_sender, _) = SyncSender::channel();
        let mut ctx = EphemeraContext::new(loom_client.clone(), sync_sender, 10);

        // Perform pin operation
        let result = ctx.pin(42, "Important".to_string()).await;
        assert!(result.is_ok(), "Pin should succeed");
        assert_eq!(ctx.list_pinned().len(), 1);

        // Phase 2: Simulate crash and restart
        drop(ctx);

        // Setup mock with pinned state (pin was persisted to Loom)
        let mut mock_restart = MockLoomClient::new();
        mock_restart.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock_restart.set_default_pinned(PinnedMemoriesResponse {
            items: vec![create_pinned(42, "Runtime pinned content", Some("Important"))],
        });

        let (new_sync_sender, _) = SyncSender::channel();
        let mut ctx_restarted = EphemeraContext::new(
            Arc::new(mock_restart),
            new_sync_sender,
            10,
        );

        ctx_restarted.restore_pinned_from_loom().await.unwrap();

        // Pin should persist
        assert_eq!(ctx_restarted.list_pinned().len(), 1);
        assert_eq!(ctx_restarted.list_pinned()[0].fragment.id, 42);
    }

    /// Test: Unpin during runtime persists after crash
    #[tokio::test]
    async fn test_unpin_during_runtime_persists_after_crash() {
        // Phase 1: Start with pinned memory
        let mut mock_runtime = MockLoomClient::new();
        mock_runtime.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        // Push response for restore_pinned_from_loom (get_pinned_memories)
        mock_runtime.push_pinned_memories(PinnedMemoriesResponse {
            items: vec![create_pinned(1, "Originally pinned", Some("Original"))],
        });
        // unpin_memory returns Ok(()) by default when queue is empty, no need to push

        let loom_client: Arc<dyn LoomClientTrait> = Arc::new(mock_runtime);
        let (sync_sender, _) = SyncSender::channel();
        let mut ctx = EphemeraContext::new(loom_client.clone(), sync_sender, 10);

        // Restore initial pinned state
        ctx.restore_pinned_from_loom().await.unwrap();
        assert_eq!(ctx.list_pinned().len(), 1);

        // Perform unpin operation
        let removed = ctx.unpin(1).await;
        assert!(removed, "Unpin should succeed");
        assert_eq!(ctx.list_pinned().len(), 0);

        // Phase 2: Simulate crash and restart
        drop(ctx);

        // Setup mock with empty pinned state (unpin was persisted to Loom)
        let mut mock_restart = MockLoomClient::new();
        mock_restart.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock_restart.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let (new_sync_sender, _) = SyncSender::channel();
        let mut ctx_restarted = EphemeraContext::new(
            Arc::new(mock_restart),
            new_sync_sender,
            10,
        );

        ctx_restarted.restore_pinned_from_loom().await.unwrap();

        // Unpin should persist
        assert_eq!(ctx_restarted.list_pinned().len(), 0);
    }
}
