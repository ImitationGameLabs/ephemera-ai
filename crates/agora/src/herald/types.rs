pub use agora_common::herald::*;

use std::collections::HashMap;
use time::OffsetDateTime;
use tokio::sync::RwLock;

/// In-memory herald registry.
#[derive(Debug, Default)]
pub struct HeraldRegistry {
    heralds: RwLock<HashMap<String, HeraldInfo>>,
}

impl HeraldRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new herald.
    pub async fn register(&self, request: RegisterHeraldRequest) -> HeraldInfo {
        let now = OffsetDateTime::now_utc();
        let info = HeraldInfo {
            id: request.id.clone(),
            description: request.description,
            status: HeraldStatus::Active,
            registered_at: now,
            last_heartbeat: now,
        };

        let mut heralds = self.heralds.write().await;
        heralds.insert(request.id, info.clone());
        info
    }

    /// Gets a herald by ID.
    pub async fn get(&self, id: &str) -> Option<HeraldInfo> {
        let heralds = self.heralds.read().await;
        heralds.get(id).cloned()
    }

    /// Lists all heralds.
    pub async fn list(&self) -> Vec<HeraldInfo> {
        let heralds = self.heralds.read().await;
        heralds.values().cloned().collect()
    }

    /// Updates herald heartbeat.
    pub async fn heartbeat(&self, id: &str) -> Option<HeartbeatResponse> {
        let mut heralds = self.heralds.write().await;
        if let Some(info) = heralds.get_mut(id) {
            let now = OffsetDateTime::now_utc();
            info.last_heartbeat = now;
            info.status = HeraldStatus::Active;
            Some(HeartbeatResponse {
                status: info.status,
                last_heartbeat: info.last_heartbeat,
            })
        } else {
            None
        }
    }

    /// Unregisters a herald.
    pub async fn unregister(&self, id: &str) -> bool {
        let mut heralds = self.heralds.write().await;
        heralds.remove(id).is_some()
    }

    /// Updates herald statuses based on heartbeat timeout.
    /// Returns list of heralds that changed status to Disconnected.
    pub async fn check_timeouts(&self, timeout_ms: i64) -> Vec<(String, HeraldStatus)> {
        let mut changed = Vec::new();
        let mut heralds = self.heralds.write().await;
        let now = OffsetDateTime::now_utc();

        for (id, info) in heralds.iter_mut() {
            // Skip already disconnected heralds
            if info.status == HeraldStatus::Disconnected {
                continue;
            }

            let elapsed_ms = (now - info.last_heartbeat).whole_milliseconds() as i64;

            if elapsed_ms > timeout_ms {
                info.status = HeraldStatus::Disconnected;
                changed.push((id.clone(), HeraldStatus::Disconnected));
            }
        }

        changed
    }
}
