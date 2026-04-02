//! Delivery state tracking for events.

use crate::config::RetryConfig;
use crate::event::{Event, EventId};
use std::collections::HashMap;
use time::OffsetDateTime;
use tokio::sync::RwLock;

/// Delivery state for a single event.
/// State is determined by `delivered_at`:
/// - None = pending (never fetched)
/// - Some(t) = delivered at time t (waiting for ack)
#[derive(Debug, Clone)]
pub struct DeliveryState {
    pub event: Event,
    /// None = pending, Some = delivered
    pub delivered_at: Option<OffsetDateTime>,
    pub retry_count: u32,
    pub next_retry_at: Option<OffsetDateTime>,
}

/// In-memory delivery state cache.
#[derive(Debug, Default)]
pub struct DeliveryCache {
    events: RwLock<HashMap<EventId, DeliveryState>>,
}

impl DeliveryCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add event as pending (delivered_at = None).
    pub async fn add_pending(&self, event: Event) {
        let state =
            DeliveryState { event, delivered_at: None, retry_count: 0, next_retry_at: None };
        self.events.write().await.insert(state.event.id, state);
    }

    /// Load events from SQLite on startup.
    pub async fn load_pending(&self, events: Vec<Event>) {
        let mut cache = self.events.write().await;
        for event in events {
            let state =
                DeliveryState { event, delivered_at: None, retry_count: 0, next_retry_at: None };
            cache.insert(state.event.id, state);
        }
    }

    /// Get events ready for delivery (pending + retries).
    /// Marks them as delivered and returns them.
    pub async fn get_deliverable(&self, limit: u32, config: &RetryConfig) -> Vec<Event> {
        let mut cache = self.events.write().await;
        let now = OffsetDateTime::now_utc();
        let mut result = Vec::new();

        for state in cache.values_mut() {
            if result.len() >= limit as usize {
                break;
            }

            let is_pending = state.delivered_at.is_none();
            let is_retry_ready = state.next_retry_at.map(|t| t <= now).unwrap_or(false);

            if is_pending {
                // Pending → Delivered
                state.delivered_at = Some(now);
                state.retry_count = 0;
                state.next_retry_at = Some(now + retry_interval(0, config));
                result.push(state.event.clone());
            } else if is_retry_ready {
                // Delivered + retry ready → fetch again
                state.retry_count += 1;
                state.next_retry_at = Some(now + retry_interval(state.retry_count, config));
                result.push(state.event.clone());
            }
        }

        result
    }

    /// Remove event (called on ack after SQLite delete succeeds).
    pub async fn remove(&self, id: EventId) -> Option<Event> {
        self.events.write().await.remove(&id).map(|s| s.event)
    }
}

fn retry_interval(retry_count: u32, config: &RetryConfig) -> time::Duration {
    let multiplier = config.multiplier as u64;

    // multiplier <= 1 means no exponential growth
    if multiplier <= 1 {
        let interval = config.base_interval_ms.min(config.max_interval_ms);
        return time::Duration::milliseconds(interval as i64);
    }

    // Use checked_pow to detect power overflow
    match multiplier.checked_pow(retry_count) {
        Some(factor) => {
            // Use saturating_mul to handle multiplication overflow
            let interval = config
                .base_interval_ms
                .saturating_mul(factor)
                .min(config.max_interval_ms);
            time::Duration::milliseconds(interval as i64)
        }
        None => {
            // Power overflow - return max interval
            time::Duration::milliseconds(config.max_interval_ms as i64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RetryConfig;
    use crate::event::{EventPriority, EventStatus};

    fn default_config() -> RetryConfig {
        RetryConfig { base_interval_ms: 5000, multiplier: 2, max_interval_ms: 300000 }
    }

    fn test_event(id: u64) -> Event {
        Event {
            id,
            event_type: "test".to_string(),
            herald_id: "test-herald".to_string(),
            payload: serde_json::json!({}),
            priority: EventPriority::Normal,
            timestamp: OffsetDateTime::now_utc(),
            status: EventStatus::Pending,
        }
    }

    // ============== retry_interval tests ==============

    #[test]
    fn test_retry_interval_first_attempt() {
        // retry_count = 0: 5000 * 2^0 = 5000ms
        let config = default_config();
        let duration = retry_interval(0, &config);
        assert_eq!(duration, time::Duration::milliseconds(5000));
    }

    #[test]
    fn test_retry_interval_exponential_growth() {
        let config = default_config();
        // retry_count = 1: 5000 * 2^1 = 10000ms
        assert_eq!(
            retry_interval(1, &config),
            time::Duration::milliseconds(10000)
        );
        // retry_count = 2: 5000 * 2^2 = 20000ms
        assert_eq!(
            retry_interval(2, &config),
            time::Duration::milliseconds(20000)
        );
        // retry_count = 5: 5000 * 2^5 = 160000ms
        assert_eq!(
            retry_interval(5, &config),
            time::Duration::milliseconds(160000)
        );
    }

    #[test]
    fn test_retry_interval_capped_at_max() {
        let config = default_config();
        // retry_count = 6: 5000 * 2^6 = 320000, capped to 300000
        assert_eq!(
            retry_interval(6, &config),
            time::Duration::milliseconds(300000)
        );
        // retry_count = 10: would be huge, capped to 300000
        assert_eq!(
            retry_interval(10, &config),
            time::Duration::milliseconds(300000)
        );
    }

    #[test]
    fn test_retry_interval_overflow_protection() {
        let config = default_config();
        // Very high retry_count would cause overflow without protection
        // With protection, should return max_interval
        assert_eq!(
            retry_interval(100, &config),
            time::Duration::milliseconds(300000)
        );
        assert_eq!(
            retry_interval(u32::MAX, &config),
            time::Duration::milliseconds(300000)
        );
    }

    #[test]
    fn test_retry_interval_multiplier_one() {
        let config = RetryConfig { base_interval_ms: 5000, multiplier: 1, max_interval_ms: 300000 };
        // With multiplier = 1, no exponential growth
        assert_eq!(
            retry_interval(0, &config),
            time::Duration::milliseconds(5000)
        );
        assert_eq!(
            retry_interval(100, &config),
            time::Duration::milliseconds(5000)
        );
    }

    #[test]
    fn test_retry_interval_multiplier_zero() {
        let config = RetryConfig { base_interval_ms: 5000, multiplier: 0, max_interval_ms: 300000 };
        // With multiplier = 0, should return min(base, max)
        assert_eq!(
            retry_interval(0, &config),
            time::Duration::milliseconds(5000)
        );
        assert_eq!(
            retry_interval(10, &config),
            time::Duration::milliseconds(5000)
        );
    }

    #[test]
    fn test_retry_interval_large_multiplier() {
        let config = RetryConfig { base_interval_ms: 1000, multiplier: 10, max_interval_ms: 60000 };
        // 1000 * 10^0 = 1000
        assert_eq!(
            retry_interval(0, &config),
            time::Duration::milliseconds(1000)
        );
        // 1000 * 10^1 = 10000
        assert_eq!(
            retry_interval(1, &config),
            time::Duration::milliseconds(10000)
        );
        // 1000 * 10^2 = 100000, capped to 60000
        assert_eq!(
            retry_interval(2, &config),
            time::Duration::milliseconds(60000)
        );
        // 10^20 would overflow, should return max
        assert_eq!(
            retry_interval(20, &config),
            time::Duration::milliseconds(60000)
        );
    }

    // ============== DeliveryCache tests ==============

    #[tokio::test]
    async fn test_delivery_cache_add_pending() {
        let cache = DeliveryCache::new();
        let event = test_event(1);
        cache.add_pending(event.clone()).await;

        let deliverable = cache.get_deliverable(10, &default_config()).await;
        assert_eq!(deliverable.len(), 1);
        assert_eq!(deliverable[0].id, 1);
    }

    #[tokio::test]
    async fn test_delivery_cache_fetch_marks_as_delivered() {
        let cache = DeliveryCache::new();
        cache.add_pending(test_event(1)).await;

        // First fetch
        let events = cache.get_deliverable(10, &default_config()).await;
        assert_eq!(events.len(), 1);

        // Immediately fetch again - should not return (not retry-ready yet)
        let events2 = cache.get_deliverable(10, &default_config()).await;
        assert_eq!(events2.len(), 0);
    }

    #[tokio::test]
    async fn test_delivery_cache_remove() {
        let cache = DeliveryCache::new();
        cache.add_pending(test_event(1)).await;

        let removed = cache.remove(1).await;
        assert!(removed.is_some());

        let deliverable = cache.get_deliverable(10, &default_config()).await;
        assert_eq!(deliverable.len(), 0);
    }

    #[tokio::test]
    async fn test_delivery_cache_remove_nonexistent() {
        let cache = DeliveryCache::new();

        let removed = cache.remove(999).await;
        assert!(removed.is_none());
    }

    #[tokio::test]
    async fn test_delivery_cache_limit_respected() {
        let cache = DeliveryCache::new();
        for i in 1..=5 {
            cache.add_pending(test_event(i)).await;
        }

        let deliverable = cache.get_deliverable(3, &default_config()).await;
        assert_eq!(deliverable.len(), 3);
    }

    #[tokio::test]
    async fn test_delivery_cache_empty() {
        let cache = DeliveryCache::new();

        let deliverable = cache.get_deliverable(10, &default_config()).await;
        assert_eq!(deliverable.len(), 0);
    }

    #[tokio::test]
    async fn test_delivery_cache_load_pending() {
        let cache = DeliveryCache::new();
        let events = vec![test_event(1), test_event(2), test_event(3)];
        cache.load_pending(events).await;

        let deliverable = cache.get_deliverable(10, &default_config()).await;
        assert_eq!(deliverable.len(), 3);
    }
}
