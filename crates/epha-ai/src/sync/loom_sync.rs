use crate::context::{fragment_log_meta, summarize_batch_log_meta};
use loom_client::memory::MemoryFragment;
use loom_client::{CreateMemoryRequest, LoomClientTrait};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Channel size for pending sync queue
const SYNC_CHANNEL_SIZE: usize = 256;

/// Maximum batch size for syncing to Loom
const MAX_BATCH_SIZE: usize = 32;

/// Sender for memory fragments to be synced to Loom
#[derive(Clone)]
pub struct SyncSender {
    sender: mpsc::Sender<MemoryFragment>,
}

impl SyncSender {
    /// Create a new sync channel and return (sender, receiver)
    pub fn channel() -> (Self, mpsc::Receiver<MemoryFragment>) {
        let (sender, receiver) = mpsc::channel(SYNC_CHANNEL_SIZE);
        (Self { sender }, receiver)
    }

    /// Send a memory fragment to the sync queue
    /// Returns immediately, does not block
    pub fn send(&self, fragment: MemoryFragment) {
        let meta = fragment_log_meta(&fragment);

        // Use try_send to avoid blocking
        match self.sender.try_send(fragment) {
            Ok(()) => {
                debug!(
                    target: "epha_ai::memory",
                    stage = "enqueued",
                    kind = meta.kind,
                    event_type = meta.event_type.as_deref().unwrap_or("-"),
                    tool_call_count = meta.tool_call_count.unwrap_or(0),
                    text_len = meta.text_len.unwrap_or(0),
                    parse_fallback = meta.parse_fallback,
                    "memory.enqueued"
                );
            }
            Err(mpsc::error::TrySendError::Full(_fragment)) => {
                // Queue is full, log warning and drop the fragment
                // This is acceptable per the design: Loom is the source of truth
                warn!(
                    target: "epha_ai::memory",
                    stage = "dropped",
                    reason = "queue_full",
                    kind = meta.kind,
                    event_type = meta.event_type.as_deref().unwrap_or("-"),
                    tool_call_count = meta.tool_call_count.unwrap_or(0),
                    text_len = meta.text_len.unwrap_or(0),
                    parse_fallback = meta.parse_fallback,
                    "memory.dropped"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Channel closed, this shouldn't happen in normal operation
                error!("Sync channel closed, cannot send memory fragment");
            }
        }
    }
}

/// Start the background sync task
/// Consumes the receiver and runs until the channel is closed
pub fn start_sync_task(
    mut receiver: mpsc::Receiver<MemoryFragment>,
    loom_client: Arc<dyn LoomClientTrait>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("Loom sync task started");
        let mut batch: Vec<MemoryFragment> = Vec::with_capacity(MAX_BATCH_SIZE);

        loop {
            // Wait for the first fragment
            match receiver.recv().await {
                Some(fragment) => {
                    batch.push(fragment);

                    // Try to collect more fragments without waiting
                    while batch.len() < MAX_BATCH_SIZE {
                        match receiver.try_recv() {
                            Ok(fragment) => batch.push(fragment),
                            Err(mpsc::error::TryRecvError::Empty) => break,
                            Err(mpsc::error::TryRecvError::Disconnected) => {
                                // Channel closed, sync remaining and exit
                                break;
                            }
                        }
                    }

                    // Sync the batch
                    sync_batch(loom_client.as_ref(), &batch).await;
                    batch.clear();
                }
                None => {
                    // Channel closed
                    info!("Sync channel closed, flushing remaining items");
                    if !batch.is_empty() {
                        sync_batch(loom_client.as_ref(), &batch).await;
                    }
                    break;
                }
            }
        }

        info!("Loom sync task stopped");
    })
}

/// Sync a batch of memory fragments to Loom
async fn sync_batch(loom_client: &dyn LoomClientTrait, fragments: &[MemoryFragment]) {
    if fragments.is_empty() {
        return;
    }

    let summary = summarize_batch_log_meta(fragments);
    debug!(
        target: "epha_ai::memory",
        stage = "syncing",
        total = summary.total,
        kind_counts = ?summary.kind_counts,
        event_type_counts = ?summary.event_type_counts,
        parse_fallback_count = summary.parse_fallback_count,
        "memory.syncing"
    );

    let request = CreateMemoryRequest::multiple(fragments.to_vec());
    match loom_client.create_memory(request).await {
        Ok(_) => {
            debug!(
                target: "epha_ai::memory",
                stage = "synced",
                total = summary.total,
                kind_counts = ?summary.kind_counts,
                event_type_counts = ?summary.event_type_counts,
                parse_fallback_count = summary.parse_fallback_count,
                "memory.synced"
            );
        }
        Err(e) => {
            error!(
                target: "epha_ai::memory",
                stage = "sync_failed",
                total = summary.total,
                kind_counts = ?summary.kind_counts,
                event_type_counts = ?summary.event_type_counts,
                parse_fallback_count = summary.parse_fallback_count,
                error = ?e,
                "memory.sync_failed"
            );
            // Per design: we accept data loss on sync failure
            // Loom is the source of truth, lost fragments will be recovered on restart
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_fragment(content: &str) -> MemoryFragment {
        MemoryFragment {
            id: 0,
            content: serde_json::to_string(&serde_json::json!({
                "text": content
            }))
            .unwrap(),
            timestamp: time::OffsetDateTime::now_utc(),
            kind: loom_client::memory::MemoryKind::Action,
        }
    }

    #[tokio::test]
    async fn test_sync_sender_send() {
        let (sender, mut receiver) = SyncSender::channel();

        let fragment = create_test_fragment("test content");
        sender.send(fragment);

        // Should receive immediately
        let received = receiver.try_recv();
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_sync_sender_drops_when_full() {
        let (sender, receiver) = mpsc::channel(2);
        let sync_sender = SyncSender { sender };

        // Fill the channel
        sync_sender.send(create_test_fragment("1"));
        sync_sender.send(create_test_fragment("2"));

        // This should be dropped without blocking
        sync_sender.send(create_test_fragment("3"));

        // Verify only 2 items in channel
        let mut receiver = receiver;
        assert!(receiver.try_recv().is_ok());
        assert!(receiver.try_recv().is_ok());
        assert!(receiver.try_recv().is_err()); // Channel empty
    }
}
