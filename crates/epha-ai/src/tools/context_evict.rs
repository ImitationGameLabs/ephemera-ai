use crate::context::EphemeraContext;
use crate::tools::AgentTool;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Tool for proactively evicting context to free token budget.
pub struct ContextEvict {
    context: Arc<Mutex<EphemeraContext>>,
}

impl ContextEvict {
    pub fn new(context: Arc<Mutex<EphemeraContext>>) -> Self {
        Self { context }
    }
}

#[async_trait]
impl AgentTool for ContextEvict {
    fn name(&self) -> &str {
        "context_evict"
    }

    fn description(&self) -> &str {
        "Evict oldest activities from context to free token budget. Use before recall-heavy operations to ensure sufficient headroom."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn call(&self, _args_json: &str) -> anyhow::Result<String> {
        let mut ctx = self.context.lock().await;
        let (evicted, freed) = ctx.evict_to_floor();

        if evicted == 0 {
            return Ok("No activities to evict.".to_string());
        }

        Ok(format!(
            "Evicted {} activities, freed ~{} tokens.",
            evicted, freed
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ContextConfig;
    use crate::sync::SyncSender;
    use loom_client::memory::{MemoryFragment, MemoryKind};
    use loom_client::mock::MockLoomClient;
    use std::sync::Arc;
    use time::OffsetDateTime;

    fn create_fragment(id: i64, content: &str) -> MemoryFragment {
        MemoryFragment {
            id,
            content: content.to_string(),
            timestamp: OffsetDateTime::now_utc(),
            kind: MemoryKind::Action,
        }
    }

    /// Helper to create a ContextEvict with a pre-populated context.
    async fn setup_evict_tool(
        config: ContextConfig,
        fragments: Vec<MemoryFragment>,
    ) -> (ContextEvict, Arc<Mutex<EphemeraContext>>) {
        let mock = MockLoomClient::new();
        let (sync_sender, _) = SyncSender::channel();
        let mut ctx = EphemeraContext::new(Arc::new(mock), sync_sender, config);
        for f in fragments {
            ctx.add_activity(f);
        }
        let ctx_arc = Arc::new(Mutex::new(ctx));
        let tool = ContextEvict::new(ctx_arc.clone());
        (tool, ctx_arc)
    }

    #[tokio::test]
    async fn evict_reports_when_nothing_to_evict() {
        // floor=5000, only a tiny activity — nothing to evict
        let config = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 5000,
            total_token_ceiling: 10_000,
            min_activities: 1,
        };
        let frag = create_fragment(1, "hi");
        let (tool, _) = setup_evict_tool(config, vec![frag]).await;

        let result = tool.call("{}").await.unwrap();
        assert!(result.contains("No activities to evict"));
    }

    #[tokio::test]
    async fn evict_reports_count_and_tokens() {
        // Very low floor so eviction definitely triggers
        let config = ContextConfig {
            max_pinned_tokens: 10,
            total_token_floor: 10,
            total_token_ceiling: 100_000,
            min_activities: 1,
        };
        let frags = vec![
            create_fragment(1, &"a".repeat(500)),
            create_fragment(2, &"b".repeat(500)),
            create_fragment(3, &"c".repeat(500)),
        ];
        let (tool, _) = setup_evict_tool(config, frags).await;

        let result = tool.call("{}").await.unwrap();
        assert!(result.contains("Evicted"), "result: {result}");
        assert!(result.contains("freed"), "result: {result}");
    }
}
