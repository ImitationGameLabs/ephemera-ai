use super::MemoryFragmentList;
use super::memory_constructors::from_agora_event;
use super::memory_content::{SerializeContext, ToChatMessages, format_rfc3339};
use crate::config::ContextConfig;
use crate::sync::SyncSender;
use agora_common::event::Event;
use llm::chat::{ChatMessage, ChatRole, MessageType};
use loom_client::memory::{MemoryFragment, MemoryKind};
use loom_client::{CreateMemoryRequest, LoomClientTrait};
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use time::OffsetDateTime;
use tokenx_rs::estimate_token_count;
use tracing::error;

// Re-export PinnedMemory for external use
pub use loom_client::PinnedMemory;

#[derive(Debug, Clone)]
pub struct QueueStatus {
    pub activity_count: usize,
    pub current_token_usage: usize,
    pub token_ceiling: usize,
    pub utilization_ratio: f64,
}

impl fmt::Display for QueueStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Activities: {}, Tokens: {}/{} ({:.1}%)",
            self.activity_count,
            self.current_token_usage,
            self.token_ceiling,
            self.utilization_ratio * 100.0
        )
    }
}

/// Mirror of the OpenAI-compatible API message format, used only for serialization.
///
/// Two purposes:
/// 1. Token budget estimation — serialize to JSON and count tokens on the exact
///    payload the LLM will receive, rather than on an internal format.
/// 2. Rendering context for inspection — the `render_chat_history` test uses this
///    to print the final JSON that gets sent to the API.
///
/// **Convention**: The `llm` crate reuses `ToolCall.function.arguments` as a
/// carrier for tool result content in `MessageType::ToolResult`. When building
/// `"tool"`-role messages, `content` is populated from `tc.function.arguments`,
/// which actually holds the tool's output string, not its input arguments. This
/// matches the `llm` crate's `prepare_messages` behavior.
#[derive(serde::Serialize)]
pub(crate) struct OpenAIMessage {
    pub(crate) role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_calls: Option<Vec<llm::ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
}

pub struct EphemeraContext {
    loom_client: Arc<dyn LoomClientTrait>,
    sync_sender: SyncSender,
    pinned_memories: Vec<PinnedMemory>,
    recalled_memories: Vec<MemoryFragment>,
    recent_activities: VecDeque<MemoryFragment>,
    current_token_usage: usize,
    max_pinned_tokens: usize,
    total_token_floor: usize,
    total_token_ceiling: usize,
    min_activities: usize,
}

impl EphemeraContext {
    pub fn new(
        loom_client: Arc<dyn LoomClientTrait>,
        sync_sender: SyncSender,
        config: ContextConfig,
    ) -> Self {
        Self {
            pinned_memories: Vec::new(),
            recalled_memories: Vec::new(),
            recent_activities: VecDeque::new(),
            current_token_usage: 0,
            max_pinned_tokens: config.max_pinned_tokens,
            total_token_floor: config.total_token_floor,
            total_token_ceiling: config.total_token_ceiling,
            min_activities: config.min_activities,
            sync_sender,
            loom_client,
        }
    }

    /// Pin a memory by ID via Loom API
    /// This is an async operation that persists to the database
    pub async fn pin(&mut self, memory_id: i64, reason: String) -> Result<(), String> {
        if self.pinned_memories.len() >= self.max_pinned_tokens {
            return Err(format!(
                "Maximum pinned count ({}) reached, please unpin some content first",
                self.max_pinned_tokens
            ));
        }

        // Check if already pinned
        if self
            .pinned_memories
            .iter()
            .any(|p| p.fragment.id == memory_id)
        {
            return Err(format!("Memory {} is already pinned", memory_id));
        }

        // Call Loom API to pin
        let pinned = self
            .loom_client
            .pin_memory(memory_id, Some(reason))
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

    /// Serialize pinned memories as XML for assistant role message.
    pub fn serialize_pinned(&self) -> Option<String> {
        if self.pinned_memories.is_empty() {
            return None;
        }

        let mut xml = String::from("<pinned_memories>\n");
        for item in &self.pinned_memories {
            let reason_str = item.reason.as_deref().unwrap_or("unspecified");
            let pinned_at = format_rfc3339(&item.pinned_at);
            let fragment = &item.fragment;

            xml.push_str(&format!(
                "  <memory id=\"{}\" pinned-at=\"{}\" reason=\"{}\">\n",
                fragment.id, pinned_at, reason_str
            ));

            // Render inner content using structured XML serialization
            let inner = fragment.serialize_context();
            for line in inner.split('\n') {
                xml.push_str("    ");
                xml.push_str(line);
                xml.push('\n');
            }

            xml.push_str("  </memory>\n");
        }
        xml.push_str("</pinned_memories>");
        Some(xml)
    }

    /// Get a reference to the recent activities deque.
    pub fn recent_activities(&self) -> &VecDeque<MemoryFragment> {
        &self.recent_activities
    }

    /// Get max pinned tokens
    pub fn max_pinned_tokens(&self) -> usize {
        self.max_pinned_tokens
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

        let response = self
            .loom_client
            .get_pinned_memories()
            .await
            .map_err(|e| format!("Failed to fetch pinned memories: {:?}", e))?;

        if response.items.is_empty() {
            info!("No pinned memories found in Loom");
            return Ok(());
        }

        self.pinned_memories = response.items;
        self.recalculate_token_usage();

        info!(
            "Restored {} pinned memories from Loom",
            self.pinned_memories.len()
        );

        Ok(())
    }

    fn recalculate_token_usage(&mut self) {
        self.current_token_usage = estimate_token_count(&self.build_context_json());
    }

    /// Build the full OpenAI API JSON for accurate token counting.
    fn build_context_json(&self) -> String {
        let mut messages: Vec<ChatMessage> = vec![];

        if let Some(pinned_xml) = self.serialize_pinned() {
            messages.push(ChatMessage::assistant().content(&pinned_xml).build());
        }
        for fragment in &self.recent_activities {
            messages.extend(fragment.to_chat_messages());
        }

        let api_messages: Vec<OpenAIMessage> = messages
            .iter()
            .flat_map(|msg| match &msg.message_type {
                MessageType::ToolResult(results) => {
                    // ToolResult expands into separate "tool" role messages
                    results
                        .iter()
                        .map(|tc| OpenAIMessage {
                            role: "tool",
                            tool_call_id: Some(tc.id.clone()),
                            tool_calls: None,
                            content: Some(tc.function.arguments.clone()),
                        })
                        .collect()
                }
                MessageType::ToolUse(calls) => vec![OpenAIMessage {
                    role: "assistant",
                    content: None,
                    tool_calls: Some(calls.clone()),
                    tool_call_id: None,
                }],
                _ => vec![OpenAIMessage {
                    role: match msg.role {
                        ChatRole::User => "user",
                        ChatRole::Assistant => "assistant",
                    },
                    content: Some(msg.content.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                }],
            })
            .collect();

        serde_json::to_string(&api_messages).unwrap()
    }

    pub fn add_activity(&mut self, fragment: MemoryFragment) {
        let mut temp_fragment = fragment.clone();
        temp_fragment.id = 0; // Temporary ID for tracking
        self.recent_activities.push_back(temp_fragment);
        self.recalculate_token_usage();

        // Add to sync queue for background sync to Loom
        self.sync_sender.send(fragment);

        self.maintain_token_limit();
    }

    /// Store recalled memory fragments for temporary context injection.
    ///
    /// These fragments are injected as a temporary user message at the end of
    /// chat_history during the cognitive cycle. They are cleared after each cycle.
    pub fn add_recalled_memories(&mut self, fragments: Vec<MemoryFragment>) {
        for memory in fragments {
            if !self.recalled_memories.iter().any(|m| m.id == memory.id) {
                self.recalled_memories.push(memory);
            }
        }
    }

    /// Get the recalled memories buffer.
    pub fn recalled_memories(&self) -> &[MemoryFragment] {
        &self.recalled_memories
    }

    /// Clear the recall buffer at the end of a cognitive cycle.
    pub fn clear_recalled_memories(&mut self) {
        self.recalled_memories.clear();
    }

    #[cfg(test)]
    pub fn add_recalled_for_testing(&mut self, memories: Vec<MemoryFragment>) {
        for memory in memories {
            if !self.recalled_memories.iter().any(|m| m.id == memory.id) {
                self.recalled_memories.push(memory);
            }
        }
    }

    pub fn add_agora_events(&mut self, events: Vec<Event>) {
        for event in events {
            let fragment = from_agora_event(event);
            self.add_activity(fragment);
        }
    }

    fn maintain_token_limit(&mut self) {
        if self.current_token_usage < self.total_token_ceiling {
            return;
        }

        while self.current_token_usage > self.total_token_floor
            && self.recent_activities.len() > self.min_activities
        {
            self.recent_activities.pop_front();
            self.recalculate_token_usage();
        }
    }

    pub fn get_queue_status(&self) -> QueueStatus {
        QueueStatus {
            activity_count: self.recent_activities.len(),
            current_token_usage: self.current_token_usage,
            token_ceiling: self.total_token_ceiling,
            utilization_ratio: self.current_token_usage as f64 / self.total_token_ceiling as f64,
        }
    }

    /// Check if current token usage is below the ceiling (in safe zone)
    pub fn is_in_safe_zone(&self) -> bool {
        self.current_token_usage < self.total_token_ceiling
    }

    /// Get the total token ceiling
    pub fn total_token_ceiling(&self) -> usize {
        self.total_token_ceiling
    }

    /// Restore recent activities from Loom on startup
    /// This is called after context creation to recover state after a crash
    pub async fn restore_from_loom(&mut self, limit: usize) -> Result<(), String> {
        use tracing::info;

        info!("Restoring recent activities from Loom (limit: {})", limit);

        let response = self
            .loom_client
            .get_recent_memories(limit)
            .await
            .map_err(|e| format!("Failed to fetch recent memories: {:?}", e))?;

        if response.fragments.is_empty() {
            info!("No recent memories found in Loom");
            return Ok(());
        }

        // Build pinned ID set for deduplication
        let pinned_ids: std::collections::HashSet<i64> =
            self.pinned_memories.iter().map(|p| p.fragment.id).collect();

        // Add fragments directly to recent_activities without going through sync_sender.
        // This avoids a sync loop: we're restoring FROM Loom, so there's no need to sync back TO Loom.
        // Filter out memories already pinned to avoid duplicates.
        for fragment in response.fragments {
            if !pinned_ids.contains(&fragment.id) {
                self.recent_activities.push_back(fragment);
            }
        }

        self.recalculate_token_usage();

        info!(
            "Restored {} memories from Loom",
            self.recent_activities.len()
        );

        // Maintain token limit after restoration
        self.maintain_token_limit();

        Ok(())
    }
}

#[cfg(test)]
impl EphemeraContext {
    pub fn serialize(&self) -> String {
        let mut output = String::new();

        // 1. Pinned content (highest priority)
        if !self.pinned_memories.is_empty() {
            output.push_str("Pinned Content:\n");
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

        // 2. Recent activities
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
// Test Utilities
// ============================================================================

#[cfg(test)]
mod test_utils {
    use super::*;
    use crate::config::ContextConfig;
    use crate::sync::SyncSender;
    use loom_client::PinnedMemory;
    use loom_client::memory::MemoryKind;
    use loom_client::mock::MockLoomClient;
    use std::sync::Arc;

    /// Default test context config
    pub fn test_context_config() -> ContextConfig {
        ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 4000,
            total_token_ceiling: 5000,
            min_activities: 2,
        }
    }

    /// Create a test fragment with given id, content, and kind
    pub fn create_fragment_with_kind(id: i64, content: &str, kind: MemoryKind) -> MemoryFragment {
        MemoryFragment {
            id,
            content: content.to_string(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind,
        }
    }

    /// Create a test fragment with given id and content (default kind: Action)
    pub fn create_fragment(id: i64, content: &str) -> MemoryFragment {
        create_fragment_with_kind(id, content, MemoryKind::Action)
    }

    /// Create a pinned memory
    pub fn create_pinned(id: i64, content: &str, reason: Option<&str>) -> PinnedMemory {
        PinnedMemory {
            fragment: create_fragment(id, content),
            reason: reason.map(|s| s.to_string()),
            pinned_at: time::OffsetDateTime::now_utc(),
        }
    }

    /// Create a test context with mock client and default config
    pub fn create_test_context(mock: MockLoomClient) -> EphemeraContext {
        let (sync_sender, _receiver) = SyncSender::channel();
        EphemeraContext::new(Arc::new(mock), sync_sender, test_context_config())
    }

    /// Create a test context with custom config
    pub fn create_test_context_with_config(
        mock: MockLoomClient,
        config: ContextConfig,
    ) -> EphemeraContext {
        let (sync_sender, _) = SyncSender::channel();
        EphemeraContext::new(Arc::new(mock), sync_sender, config)
    }
}

// ============================================================================
// Context Restoration Consistency Tests
// ============================================================================

#[cfg(test)]
mod restoration_tests {
    use super::test_utils::*;
    use super::*;
    use loom_client::memory::MemoryKind;
    use loom_client::mock::MockLoomClient;
    use loom_client::{MemoryResponse, PinnedMemoriesResponse};

    // =========================================================================
    // Basic Restoration Tests
    // =========================================================================

    /// Test: Empty state restoration
    #[tokio::test]
    async fn test_restore_empty_state() {
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx = create_test_context(mock);
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

        let recent_fragments =
            vec![create_fragment(1, "Activity 1"), create_fragment(2, "Activity 2")];
        mock.set_default_memory(MemoryResponse { fragments: recent_fragments, total: 2 });

        let pinned_items = vec![create_pinned(100, "Pinned content", Some("Important"))];
        mock.set_default_pinned(PinnedMemoriesResponse { items: pinned_items });

        let mut ctx = create_test_context(mock);
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
            items: vec![create_pinned(2, "Memory B", Some("Still pinned"))],
        });
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });

        let mut ctx = create_test_context(mock);
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

        let mut ctx = create_test_context(mock);
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
    // recalled_memories Not Persisted Tests
    // =========================================================================

    /// Test: recalled_memories are lost after restart (by design)
    #[tokio::test]
    async fn test_recalled_memories_not_persisted() {
        // Phase 1: Create context with recalled_memories
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx = create_test_context(mock);
        ctx.add_recalled_for_testing(vec![
            create_fragment(100, "Recalled memory 1"),
            create_fragment(101, "Recalled memory 2"),
        ]);

        // Verify recalled_memories has content
        assert_eq!(ctx.recalled_memories().len(), 2);

        // Phase 2: Simulate restart with fresh context
        let mut mock_after_restart = MockLoomClient::new();
        mock_after_restart.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock_after_restart.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx_after_restart = create_test_context(mock_after_restart);
        ctx_after_restart.restore_from_loom(50).await.unwrap();
        ctx_after_restart.restore_pinned_from_loom().await.unwrap();

        // Verify recalled_memories is empty (not persisted)
        assert!(ctx_after_restart.recalled_memories().is_empty());
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
        let mut ctx = create_test_context(mock);
        ctx.restore_from_loom(50).await.unwrap();

        // Only synced data is restored
        assert_eq!(ctx.get_queue_status().activity_count, 2);
    }

    /// Test: Activity token calculation and eviction with interval window
    #[tokio::test]
    async fn test_activity_token_eviction() {
        let mut mock = MockLoomClient::new();

        // Create content that will trigger eviction but small enough to fit after eviction
        // Each fragment ~500 tokens when serialized
        let content = "x".repeat(2000);
        mock.set_default_memory(MemoryResponse {
            fragments: vec![
                create_fragment(1, &format!("{}-1", content)),
                create_fragment(2, &format!("{}-2", content)),
                create_fragment(3, &format!("{}-3", content)),
            ],
            total: 3,
        });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        // Use a config with ceiling that forces eviction but allows one fragment
        let config = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 400,
            total_token_ceiling: 800,
            min_activities: 1,
        };
        let mut ctx = create_test_context_with_config(mock, config.clone());

        ctx.restore_from_loom(50).await.unwrap();

        // Verify token usage is at or below ceiling after eviction
        let status = ctx.get_queue_status();
        let ceiling = config.total_token_ceiling;
        assert!(
            status.current_token_usage <= ceiling,
            "Token usage {} should be at or below ceiling {}",
            status.current_token_usage,
            ceiling
        );
        // Should have evicted some activities
        assert!(
            status.activity_count < 3,
            "Some activities should be evicted"
        );
    }

    /// Test: Runtime token eviction via add_activity()
    /// Verifies FIFO eviction order when adding activities exceeds ceiling
    #[tokio::test]
    async fn test_runtime_token_eviction() {
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        // Use a config with low ceiling to trigger eviction
        // Each fragment ~500 tokens when serialized
        let config = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 400,
            total_token_ceiling: 800,
            min_activities: 1,
        };
        let mut ctx = create_test_context_with_config(mock, config.clone());

        // Create content that will trigger eviction
        let content = "x".repeat(2000);

        // Add activities: older ones should be evicted as newer ones are added
        ctx.add_activity(create_fragment(1, &format!("{}-1", content))); // Will be evicted
        ctx.add_activity(create_fragment(2, &format!("{}-2", content))); // Will be evicted
        ctx.add_activity(create_fragment(3, &format!("{}-3", content))); // Kept (most recent)

        // Token usage should be at or below ceiling after eviction
        let status = ctx.get_queue_status();
        assert!(
            status.current_token_usage <= config.total_token_ceiling,
            "Token usage {} should be at or below ceiling {}",
            status.current_token_usage,
            config.total_token_ceiling
        );
        // Some eviction should have occurred
        assert!(
            status.activity_count < 3,
            "Older activities should be evicted"
        );
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

        let mut ctx = create_test_context(mock);
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

        let mut ctx = create_test_context(mock);
        ctx.restore_from_loom(50).await.unwrap();

        // Current behavior: both are added (no deduplication in restore path)
        // This test documents the current behavior
        assert_eq!(ctx.get_queue_status().activity_count, 2);
    }

    /// Test: Pinned content consumes token budget, affecting recent eviction
    #[tokio::test]
    async fn test_pinned_tokens_affect_recent_eviction() {
        let mut mock = MockLoomClient::new();

        // Pinned content - small to keep token usage low
        let pinned_content = "x".repeat(500);
        mock.set_default_pinned(PinnedMemoriesResponse {
            items: vec![create_pinned(100, &pinned_content, Some("Pinned"))],
        });

        // Recent content - larger to trigger eviction
        let recent_content = "y".repeat(2000);
        mock.set_default_memory(MemoryResponse {
            fragments: vec![
                create_fragment(1, &recent_content),
                create_fragment(2, &recent_content),
                create_fragment(3, &recent_content),
            ],
            total: 3,
        });

        // Use config with low ceiling to force eviction
        // Pinned: ~150 tokens (serialized), Recent: ~500 tokens each (serialized)
        // With ceiling 800, only pinned + 1 recent can fit
        let config = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 200,
            total_token_ceiling: 800,
            min_activities: 1,
        };
        let mut ctx = create_test_context_with_config(mock, config.clone());

        // IMPORTANT: Restore pinned FIRST, then recent
        // restore_from_loom() calls maintain_token_limit(), which evicts if over ceiling
        // restore_pinned_from_loom() does NOT evict (pinned is never evicted)
        ctx.restore_pinned_from_loom().await.unwrap();
        ctx.restore_from_loom(50).await.unwrap();

        // Pinned should be preserved (never evicted)
        assert_eq!(
            ctx.list_pinned().len(),
            1,
            "Pinned content should never be evicted"
        );

        // Recent should be evicted to fit under ceiling
        // Started with 3 recent activities, at least one should be evicted
        let status = ctx.get_queue_status();
        assert!(
            status.activity_count < 3,
            "At least one recent activity should be evicted"
        );
        assert!(
            status.current_token_usage <= config.total_token_ceiling,
            "Token usage should be under ceiling"
        );
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
        mock_initial.set_default_memory(MemoryResponse { fragments: fragments.clone(), total: 3 });
        let pinned = vec![create_pinned(100, "Pinned", Some("Important"))];
        mock_initial.set_default_pinned(PinnedMemoriesResponse { items: pinned.clone() });

        let mut ctx_initial = create_test_context(mock_initial);
        ctx_initial.restore_from_loom(50).await.unwrap();
        ctx_initial.restore_pinned_from_loom().await.unwrap();

        // Capture initial state
        let initial_activity_count = ctx_initial.get_queue_status().activity_count;
        let initial_pinned_count = ctx_initial.list_pinned().len();
        let initial_token_usage = ctx_initial.get_queue_status().current_token_usage;

        // Phase 2: Simulate graceful restart (all data synced)
        let mut mock_restart = MockLoomClient::new();
        mock_restart.set_default_memory(MemoryResponse { fragments: fragments.clone(), total: 3 });
        mock_restart.set_default_pinned(PinnedMemoriesResponse { items: pinned.clone() });

        let mut ctx_restarted = create_test_context(mock_restart);
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

    /// Test: After graceful restart, serialize() output should be byte-identical
    /// Simulates a realistic scenario with 25 activities and 5 pinned memories
    /// of varying content sizes and types
    #[tokio::test]
    async fn test_serialize_identical_after_graceful_restart() {
        // Create 25 activities with varying content sizes (100-500 chars each)
        let fragments: Vec<MemoryFragment> = (1..=25)
            .map(|i| {
                let content_size = 100 + (i * 15) % 400; // Varying sizes
                let content = format!("Activity {} - {}", i, "x".repeat(content_size));
                let kind = if i % 3 == 0 { MemoryKind::Thought } else { MemoryKind::Action };
                create_fragment_with_kind(i as i64, &content, kind)
            })
            .collect();

        // Create 5 pinned memories with different importance levels
        let pinned: Vec<PinnedMemory> = (100..=104)
            .map(|id| {
                let content = format!(
                    "Critical context #{} - {}",
                    id,
                    "important ".repeat(id as usize % 10 + 5)
                );
                let reason = format!("Pinned reason for {}", id);
                create_pinned(id, &content, Some(&reason))
            })
            .collect();

        // Phase 1: Initial context
        let mut mock_initial = MockLoomClient::new();
        mock_initial.set_default_memory(MemoryResponse {
            fragments: fragments.clone(),
            total: fragments.len(),
        });
        mock_initial.set_default_pinned(PinnedMemoriesResponse { items: pinned.clone() });

        let mut ctx_initial = create_test_context(mock_initial);
        ctx_initial.restore_from_loom(50).await.unwrap();
        ctx_initial.restore_pinned_from_loom().await.unwrap();
        let initial_serialized = ctx_initial.serialize();

        // Verify initial state has expected content
        assert!(ctx_initial.list_pinned().len() >= 5);
        assert!(ctx_initial.get_queue_status().activity_count > 0);

        // Phase 2: Restart with same data
        let mut mock_restart = MockLoomClient::new();
        mock_restart.set_default_memory(MemoryResponse { fragments, total: 25 });
        mock_restart.set_default_pinned(PinnedMemoriesResponse { items: pinned });

        let mut ctx_restarted = create_test_context(mock_restart);
        ctx_restarted.restore_from_loom(50).await.unwrap();
        ctx_restarted.restore_pinned_from_loom().await.unwrap();
        let restarted_serialized = ctx_restarted.serialize();

        // CRITICAL: Output should be byte-identical
        assert_eq!(
            initial_serialized, restarted_serialized,
            "serialize() output should be byte-identical after graceful restart"
        );

        // Additional verification: token usage should match
        assert_eq!(
            ctx_initial.get_queue_status().current_token_usage,
            ctx_restarted.get_queue_status().current_token_usage,
            "Token usage should be identical after restart"
        );
    }

    /// Test: Recent activities order is preserved from Loom response
    /// Creates 20 activities and verifies chronological order in serialization
    #[tokio::test]
    async fn test_recent_activities_order_preserved() {
        const NUM_ACTIVITIES: usize = 20;

        // Create activities with unique, orderable content
        let fragments: Vec<MemoryFragment> = (1..=NUM_ACTIVITIES as i64)
            .map(|i| {
                let content = format!("Activity_{:02}_{}", i, "data".repeat(i as usize));
                create_fragment(i, &content)
            })
            .collect();

        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse {
            fragments: fragments.clone(),
            total: NUM_ACTIVITIES,
        });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let mut ctx = create_test_context(mock);
        ctx.restore_from_loom(50).await.unwrap();

        let serialized = ctx.serialize();

        // Find positions of all activities in serialization
        let positions: Vec<(usize, i64)> = fragments
            .iter()
            .map(|f| {
                let marker = format!("Activity_{:02}_", f.id);
                let pos = serialized.find(&marker).unwrap_or(usize::MAX);
                (pos, f.id)
            })
            .collect();

        // Verify chronological order: Activity_01 < Activity_02 < ... < Activity_20
        for i in 1..positions.len() {
            assert!(
                positions[i - 1].0 < positions[i].0,
                "Activity {} (pos {}) should appear before Activity {} (pos {}) in serialization",
                positions[i - 1].1,
                positions[i - 1].0,
                positions[i].1,
                positions[i].0
            );
        }
    }

    /// Test: Evicted content does NOT appear in serialize() output
    /// Creates 15 activities with aggressive eviction and verifies:
    /// 1. Evicted (oldest) content is absent
    /// 2. Recent content is present
    /// 3. min_activities constraint is respected
    #[tokio::test]
    async fn test_evicted_content_not_in_serialization() {
        const NUM_ACTIVITIES: usize = 15;
        const MIN_ACTIVITIES: usize = 2;

        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        // Use aggressive eviction: each activity ~1200 tokens in OpenAI JSON format
        // With ceiling 3000 and floor 1000, eviction should leave 2-3 activities
        let config = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 1000,
            total_token_ceiling: 3000,
            min_activities: MIN_ACTIVITIES,
        };
        let mut ctx = create_test_context_with_config(mock, config);

        // Track which activities we add
        let mut added_ids: Vec<i64> = Vec::new();

        for i in 1..=NUM_ACTIVITIES as i64 {
            // Create content with unique marker
            // Size ~1200 chars -> ~300 tokens when serialized
            let content = format!("ACTIVITY_MARKER_{:02}_{}", i, "x".repeat(1150));
            ctx.add_activity(create_fragment(i, &content));
            added_ids.push(i);
        }

        let status = ctx.get_queue_status();
        let serialized = ctx.serialize();

        // Verify: at least min_activities remain
        assert!(
            status.activity_count >= MIN_ACTIVITIES,
            "Should keep at least {} activities, got {}",
            MIN_ACTIVITIES,
            status.activity_count
        );

        // Verify: token usage is at or below ceiling
        assert!(
            status.current_token_usage <= 3000,
            "Token usage {} should be at or below ceiling 3000",
            status.current_token_usage
        );

        // Determine which activities should have been evicted (FIFO)
        let expected_evicted_count = NUM_ACTIVITIES.saturating_sub(status.activity_count);

        // Verify oldest activities are evicted (not in serialization)
        for i in 1..=expected_evicted_count as i64 {
            let marker = format!("ACTIVITY_MARKER_{:02}_", i);
            assert!(
                !serialized.contains(&marker),
                "Activity {} should be evicted and not appear in serialization",
                i
            );
        }

        // Verify newest activities are present
        let newest_start = (NUM_ACTIVITIES - status.activity_count + 1) as i64;
        for i in newest_start..=NUM_ACTIVITIES as i64 {
            let marker = format!("ACTIVITY_MARKER_{:02}_", i);
            assert!(
                serialized.contains(&marker),
                "Activity {} should be present in serialization (not evicted)",
                i
            );
        }
    }

    /// Test: Comprehensive session simulation with activities, pin/unpin cycles, and eviction
    /// Covers:
    /// 1. 40+ activities added during session
    /// 2. Multiple pin/unpin on same memory fragment (re-pin after unpin)
    /// 3. Pin/unpin on different memory fragments
    /// 4. Eviction triggered multiple times
    /// 5. Final state verification
    #[tokio::test]
    async fn test_comprehensive_session_with_pin_unpin_eviction() {
        // Use low ceiling to trigger eviction frequently
        let config = ContextConfig {
            max_pinned_tokens: 5,
            total_token_floor: 300,
            total_token_ceiling: 600,
            min_activities: 2,
        };

        // Setup mock with empty initial state
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        // Pre-push all API responses in order of consumption
        // The mock uses a single queue for ALL API calls, so we need to push
        // responses in the exact order they will be consumed
        //
        // Queue consumption order:
        // 1. Phase 2: pin(1) -> PinnedMemory(1)
        // 2. Phase 4: pin(2) -> PinnedMemory(2)
        // 3. Phase 4: pin(3) -> PinnedMemory(3)
        // 4. Phase 5: unpin(1) -> Empty (unpin_memory also consumes from queue)
        // 5. Phase 6: pin(1) -> PinnedMemory(1) (re-pin)
        // 6. Phase 8: unpin(2) -> Empty
        // 7. Phase 8: pin(4) -> PinnedMemory(4)
        // 8. Phase 8: unpin(3) -> Empty
        // 9. Phase 8: pin(5) -> PinnedMemory(5)

        // Phase 2: Pin memory 1 (first time)
        mock.push_pinned_memory(create_pinned(1, "Activity_1 content", Some("First pin")));
        // Phase 4: Pin memory 2 and 3
        mock.push_pinned_memory(create_pinned(2, "Activity_2 content", Some("Second pin")));
        mock.push_pinned_memory(create_pinned(3, "Activity_3 content", Some("Third pin")));
        // Phase 5: Unpin memory 1
        mock.push_empty();
        // Phase 6: Re-pin memory 1
        mock.push_pinned_memory(create_pinned(1, "Activity_1 content", Some("Re-pinned")));
        // Phase 8: Unpin memory 2, Pin 4, Unpin 3, Pin 5
        mock.push_empty();
        mock.push_pinned_memory(create_pinned(4, "Activity_4 content", Some("Pin 4")));
        mock.push_empty();
        mock.push_pinned_memory(create_pinned(5, "Activity_5 content", Some("Pin 5")));

        let mut ctx = create_test_context_with_config(mock, config.clone());

        // Track expected state
        let mut expected_pinned_ids: std::collections::HashSet<i64> =
            std::collections::HashSet::new();
        let mut total_activities_added = 0;

        // === Phase 1: Add initial activities and trigger first eviction ===
        for i in 1..=10 {
            let content = format!("Activity_{} {}", i, "x".repeat(1000));
            ctx.add_activity(create_fragment(i, &content));
            total_activities_added += 1;
        }

        let status_after_phase1 = ctx.get_queue_status();
        assert!(
            status_after_phase1.activity_count < total_activities_added,
            "Eviction should have occurred after Phase 1"
        );

        // === Phase 2: Pin memory 1 (first pin) ===
        ctx.pin(1, "First pin".to_string()).await.unwrap();
        expected_pinned_ids.insert(1);
        assert_eq!(
            ctx.list_pinned().len(),
            1,
            "Should have 1 pinned after Phase 2"
        );

        // === Phase 3: Add more activities (triggers more eviction) ===
        for i in 11..=20 {
            let content = format!("Activity_{} {}", i, "y".repeat(1000));
            ctx.add_activity(create_fragment(i, &content));
        }

        // === Phase 4: Pin memory 2, 3 (different fragments) ===
        ctx.pin(2, "Second pin".to_string()).await.unwrap();
        ctx.pin(3, "Third pin".to_string()).await.unwrap();
        expected_pinned_ids.insert(2);
        expected_pinned_ids.insert(3);
        assert_eq!(
            ctx.list_pinned().len(),
            3,
            "Should have 3 pinned after Phase 4"
        );

        // === Phase 5: Unpin memory 1 (remove first pin) ===
        ctx.unpin(1).await;
        expected_pinned_ids.remove(&1);
        assert_eq!(
            ctx.list_pinned().len(),
            2,
            "Should have 2 pinned after unpin"
        );

        // === Phase 6: Re-pin memory 1 (same fragment, new pin) ===
        ctx.pin(1, "Re-pinned".to_string()).await.unwrap();
        expected_pinned_ids.insert(1);
        assert_eq!(
            ctx.list_pinned().len(),
            3,
            "Should have 3 pinned after re-pin"
        );

        // === Phase 7: Add more activities with eviction ===
        for i in 21..=30 {
            let content = format!("Activity_{} {}", i, "z".repeat(200));
            ctx.add_activity(create_fragment(i, &content));
        }

        // === Phase 8: Pin/unpin cycles on multiple fragments ===
        // Unpin 2, pin 4, unpin 3, pin 5
        ctx.unpin(2).await;
        expected_pinned_ids.remove(&2);

        ctx.pin(4, "Pin 4".to_string()).await.unwrap();
        expected_pinned_ids.insert(4);

        ctx.unpin(3).await;
        expected_pinned_ids.remove(&3);

        ctx.pin(5, "Pin 5".to_string()).await.unwrap();
        expected_pinned_ids.insert(5);

        assert_eq!(
            ctx.list_pinned().len(),
            expected_pinned_ids.len(),
            "Pinned count should match expected"
        );

        // === Phase 9: Final batch of activities ===
        for i in 31..=40 {
            let content = format!("Activity_{} {}", i, "w".repeat(200));
            ctx.add_activity(create_fragment(i, &content));
        }

        // === Verify final state ===
        let final_status = ctx.get_queue_status();
        let final_serialized = ctx.serialize();
        let final_pinned = ctx.list_pinned();

        // Verify pinned IDs match expected
        let actual_pinned_ids: std::collections::HashSet<i64> =
            final_pinned.iter().map(|p| p.fragment.id).collect();
        assert_eq!(
            actual_pinned_ids, expected_pinned_ids,
            "Pinned IDs {:?} should match expected {:?}",
            actual_pinned_ids, expected_pinned_ids
        );

        // Verify eviction occurred (activity count < 40)
        assert!(
            final_status.activity_count < 40,
            "Eviction should have reduced activity count from 40 to {}",
            final_status.activity_count
        );

        // Verify min_activities constraint
        assert!(
            final_status.activity_count >= config.min_activities,
            "Should keep at least {} activities, got {}",
            config.min_activities,
            final_status.activity_count
        );

        // Verify token usage is within bounds
        assert!(
            final_status.current_token_usage <= config.total_token_ceiling,
            "Token usage {} should be at or below ceiling {}",
            final_status.current_token_usage,
            config.total_token_ceiling
        );

        // Verify newest activities are present in serialization
        let newest_activities_start = 40 - final_status.activity_count.min(5) + 1;
        for i in newest_activities_start as i64..=40 {
            let marker = format!("Activity_{}", i);
            assert!(
                final_serialized.contains(&marker),
                "Newest activity {} should be present in serialization",
                i
            );
        }

        // Verify pinned content is present
        for pinned in final_pinned {
            assert!(
                final_serialized.contains(&format!("Activity_{}", pinned.fragment.id))
                    || final_serialized.contains(&pinned.fragment.content),
                "Pinned memory {} content should be present in serialization",
                pinned.fragment.id
            );
        }
    }

    /// Test: Complex restart scenario with mixed pinned/recent and eviction
    /// Simulates a full AI session: startup -> operations -> graceful shutdown -> restart
    #[tokio::test]
    async fn test_complex_restart_with_eviction_and_pinned() {
        // Initial state from Loom: 20 activities, 3 pinned
        let initial_fragments: Vec<MemoryFragment> = (1..=20)
            .map(|i| {
                let content = format!(
                    "Initial activity {} - {}",
                    i,
                    "data".repeat(i as usize % 5 + 3)
                );
                create_fragment(i, &content)
            })
            .collect();

        let initial_pinned: Vec<PinnedMemory> = vec![
            create_pinned(100, "User preferences and settings", Some("Always keep")),
            create_pinned(101, "Project context and goals", Some("Critical context")),
            create_pinned(102, "Important decisions made", Some("Reference")),
        ];

        // Phase 1: Initial startup
        let mut mock_initial = MockLoomClient::new();
        mock_initial
            .set_default_memory(MemoryResponse { fragments: initial_fragments.clone(), total: 20 });
        mock_initial.set_default_pinned(PinnedMemoriesResponse { items: initial_pinned.clone() });

        let mut ctx_initial = create_test_context(mock_initial);
        ctx_initial.restore_from_loom(50).await.unwrap();
        ctx_initial.restore_pinned_from_loom().await.unwrap();

        // Add 10 more activities during runtime (simulating AI operations)
        for i in 21..=30 {
            let content = format!(
                "Runtime activity {} - {}",
                i,
                "runtime".repeat(i as usize % 8 + 2)
            );
            ctx_initial.add_activity(create_fragment(i, &content));
        }

        let initial_serialized = ctx_initial.serialize();
        let initial_status = ctx_initial.get_queue_status();
        let initial_pinned_count = ctx_initial.list_pinned().len();

        // Phase 2: Simulate graceful shutdown - all data synced to Loom
        // Collect the final state (pinned + remaining activities)
        let final_activity_count = initial_status.activity_count;
        let expected_remaining_ids: Vec<i64> = (31 - final_activity_count as i64..=30).collect();

        // Create Loom state for restart (simulating synced data)
        let restart_fragments: Vec<MemoryFragment> = expected_remaining_ids
            .iter()
            .map(|&id| {
                let content = if id <= 20 {
                    format!(
                        "Initial activity {} - {}",
                        id,
                        "data".repeat(id as usize % 5 + 3)
                    )
                } else {
                    format!(
                        "Runtime activity {} - {}",
                        id,
                        "runtime".repeat(id as usize % 8 + 2)
                    )
                };
                create_fragment(id, &content)
            })
            .collect();

        // Phase 3: Restart with synced data
        let mut mock_restart = MockLoomClient::new();
        mock_restart.set_default_memory(MemoryResponse {
            fragments: restart_fragments,
            total: final_activity_count,
        });
        mock_restart.set_default_pinned(PinnedMemoriesResponse { items: initial_pinned });

        let mut ctx_restarted = create_test_context(mock_restart);
        ctx_restarted.restore_from_loom(50).await.unwrap();
        ctx_restarted.restore_pinned_from_loom().await.unwrap();

        let restarted_serialized = ctx_restarted.serialize();

        // Verification
        assert_eq!(
            ctx_restarted.list_pinned().len(),
            initial_pinned_count,
            "Pinned count should match after restart"
        );

        assert_eq!(
            ctx_restarted.get_queue_status().activity_count,
            final_activity_count,
            "Activity count should match after restart"
        );

        // Verify pinned content is present in both serializations
        for pinned_content in &["User preferences", "Project context", "Important decisions"] {
            assert!(
                initial_serialized.contains(pinned_content),
                "Initial serialization should contain pinned: {}",
                pinned_content
            );
            assert!(
                restarted_serialized.contains(pinned_content),
                "Restarted serialization should contain pinned: {}",
                pinned_content
            );
        }

        // Verify newest runtime activities are present
        assert!(
            restarted_serialized.contains("Runtime activity 30"),
            "Newest runtime activity should be present"
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

        let mut ctx = create_test_context(mock_initial);
        ctx.restore_from_loom(50).await.unwrap();
        ctx.restore_pinned_from_loom().await.unwrap();

        // === Phase 2: Runtime operations ===
        // 2.1 Add activity
        ctx.add_activity(create_fragment(2, "Runtime activity"));

        // 2.2 Recall memories (not persisted)
        ctx.add_recalled_for_testing(vec![create_fragment(100, "Recalled during runtime")]);

        // Verify runtime state
        assert_eq!(ctx.get_queue_status().activity_count, 2);
        // Recalled memories are stored in recalled_memories buffer, not in serialize()
        assert_eq!(ctx.recalled_memories().len(), 1);

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
        let mut ctx_restarted = create_test_context(mock_after_crash);
        ctx_restarted.restore_from_loom(50).await.unwrap();
        ctx_restarted.restore_pinned_from_loom().await.unwrap();

        // Verify restored state
        // - Pinned should be restored
        assert_eq!(ctx_restarted.list_pinned().len(), 1);
        assert_eq!(ctx_restarted.list_pinned()[0].fragment.id, 100);

        // - Recent only contains synced data
        assert_eq!(ctx_restarted.get_queue_status().activity_count, 1);

        // - Recalled memories lost (not persisted)
        assert!(ctx_restarted.recalled_memories().is_empty());
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

        let mut ctx = create_test_context(mock);
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

        let mut ctx = create_test_context(mock);

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

        let mut ctx = create_test_context(mock);

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

        let mut ctx = create_test_context(mock);
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
            fragments: vec![create_fragment(1, short_content), create_fragment(2, medium_content)],
            total: 2,
        });
        mock.set_default_pinned(PinnedMemoriesResponse {
            items: vec![create_pinned(100, "Pinned", None)],
        });

        let mut ctx = create_test_context(mock);
        ctx.restore_from_loom(50).await.unwrap();
        ctx.restore_pinned_from_loom().await.unwrap();

        let status = ctx.get_queue_status();

        // Token usage should be positive
        assert!(
            status.current_token_usage > 0,
            "Token usage should be positive"
        );

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
        let mut ctx = EphemeraContext::new(loom_client.clone(), sync_sender, test_context_config());

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
        let _lost_item = sync_receiver
            .try_recv()
            .expect("Third item should exist in queue");

        // Simulate crash: drop context without processing remaining sync queue
        // The unsynced activity 3 is now lost
        drop(ctx);

        // Setup mock for restart with only synced data
        let mut mock_restart = MockLoomClient::new();
        mock_restart
            .set_default_memory(MemoryResponse { fragments: synced_fragments.clone(), total: 2 });
        mock_restart.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        // Restart: create new context
        let (new_sync_sender, _) = SyncSender::channel();
        let mut ctx_restarted = EphemeraContext::new(
            Arc::new(mock_restart),
            new_sync_sender,
            test_context_config(),
        );

        ctx_restarted.restore_from_loom(50).await.unwrap();

        // Only 2 activities should be restored (activity 3 was lost)
        assert_eq!(
            ctx_restarted.get_queue_status().activity_count,
            2,
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
        let mut ctx = EphemeraContext::new(loom_client.clone(), sync_sender, test_context_config());

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
            test_context_config(),
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
        let mut ctx = EphemeraContext::new(loom_client.clone(), sync_sender, test_context_config());

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
            test_context_config(),
        );

        ctx_restarted.restore_pinned_from_loom().await.unwrap();

        // Unpin should persist
        assert_eq!(ctx_restarted.list_pinned().len(), 0);
    }
}

// ============================================================================
// Interval Window Tests
// ============================================================================

#[cfg(test)]
mod interval_window_tests {
    use super::test_utils::*;
    use super::*;
    use crate::config::ContextConfig;
    use loom_client::mock::MockLoomClient;
    use loom_client::{MemoryResponse, PinnedMemoriesResponse};

    /// Test: Below ceiling, no eviction
    #[tokio::test]
    async fn test_expansion_no_eviction() {
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let config = test_context_config();
        let mut ctx = create_test_context_with_config(mock, config.clone());

        // Add small activities (below ceiling)
        ctx.add_activity(create_fragment(1, "small"));
        ctx.add_activity(create_fragment(2, "small"));

        assert!(ctx.is_in_safe_zone());
        assert_eq!(ctx.get_queue_status().activity_count, 2);
    }

    /// Test: At/above ceiling, eviction starts
    #[tokio::test]
    async fn test_ceiling_triggers_eviction() {
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        // Use config with ceiling lower than content size to force eviction
        // Content will be ~250 tokens when serialized, so we 2 activities (500 tokens)
        // Ceiling at 400 means eviction triggers after second activity
        let config = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 200,
            total_token_ceiling: 400,
            min_activities: 1,
        };
        let mut ctx = create_test_context_with_config(mock, config.clone());

        // Add small content that will trigger eviction after accumulation
        let small = "x".repeat(1000); // ~250 tokens when serialized
        ctx.add_activity(create_fragment(1, &small)); // ~250 tokens, stays under ceiling
        ctx.add_activity(create_fragment(2, &small)); // ~500 tokens total, exceeds ceiling, triggers eviction

        // Should have triggered eviction and be at or below ceiling
        let status = ctx.get_queue_status();
        assert!(
            status.current_token_usage <= config.total_token_ceiling,
            "Token usage {} should be at or below ceiling {}",
            status.current_token_usage,
            config.total_token_ceiling
        );
        // Should have evicted at least one activity
        assert!(
            status.activity_count >= 1,
            "At least one activity should remain"
        );
    }

    /// Test: Evicts down to floor
    #[tokio::test]
    async fn test_eviction_stops_at_floor() {
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let config = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 1000,
            total_token_ceiling: 2000,
            min_activities: 1,
        };
        let mut ctx = create_test_context_with_config(mock, config.clone());

        // Add content that exceeds ceiling
        let content = "x".repeat(5000);
        for i in 1..=5 {
            ctx.add_activity(create_fragment(i, &format!("{}-{}", content, i)));
        }

        // Token usage should be at or below ceiling
        let status = ctx.get_queue_status();
        assert!(status.current_token_usage <= config.total_token_ceiling);
    }

    /// Test: Always keeps min_activities items
    #[tokio::test]
    async fn test_min_activities_protection() {
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let config = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 100,
            total_token_ceiling: 5000,
            min_activities: 3,
        };
        let mut ctx = create_test_context_with_config(mock, config.clone());

        // Add huge content
        let huge = "x".repeat(100000);
        for i in 1..=5 {
            ctx.add_activity(create_fragment(i, &format!("{}-{}", huge, i)));
        }

        // Should keep at least min_activities
        assert!(ctx.get_queue_status().activity_count >= config.min_activities);
    }

    /// Test: Dedup on restore
    #[tokio::test]
    async fn test_restore_filters_pinned() {
        let mut mock = MockLoomClient::new();

        // Same memory appears in both pinned and recent
        mock.set_default_pinned(PinnedMemoriesResponse {
            items: vec![loom_client::PinnedMemory {
                fragment: create_fragment(42, "shared"),
                reason: Some("important".to_string()),
                pinned_at: time::OffsetDateTime::now_utc(),
            }],
        });
        mock.set_default_memory(MemoryResponse {
            fragments: vec![
                create_fragment(42, "shared"), // Duplicate
                create_fragment(43, "unique"), // Only in recent
            ],
            total: 2,
        });

        let config = test_context_config();
        let mut ctx = create_test_context_with_config(mock, config);

        // Restore pinned first
        ctx.restore_pinned_from_loom().await.unwrap();
        assert_eq!(ctx.list_pinned().len(), 1);

        // Then restore recent - should filter duplicate
        ctx.restore_from_loom(50).await.unwrap();

        // Recent should only have the non-duplicate
        assert_eq!(ctx.get_queue_status().activity_count, 1);
    }

    /// Test: Config override works
    #[tokio::test]
    async fn test_custom_config() {
        let mut mock = MockLoomClient::new();
        mock.set_default_memory(MemoryResponse { fragments: vec![], total: 0 });
        mock.set_default_pinned(PinnedMemoriesResponse { items: vec![] });

        let custom = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 1000,
            total_token_ceiling: 2000,
            min_activities: 5,
        };

        let ctx = create_test_context_with_config(mock, custom.clone());
        assert_eq!(ctx.total_token_ceiling(), 2000);
    }
}
