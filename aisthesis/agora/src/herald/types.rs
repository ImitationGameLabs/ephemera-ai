//! Herald types for Agora event hub.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::OffsetDateTime;
use tokio::sync::RwLock;

/// Herald status based on heartbeat health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HeraldStatus {
    /// Herald is active and sending heartbeats.
    #[default]
    Active,
    /// Herald missed one heartbeat (degraded but functional).
    Degraded,
    /// Herald missed multiple heartbeats (disconnected).
    Disconnected,
}

/// Information about a registered herald.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeraldInfo {
    /// Unique herald identifier.
    pub id: String,
    /// Herald description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Current herald status.
    pub status: HeraldStatus,
    /// Registration timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub registered_at: OffsetDateTime,
    /// Last heartbeat timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub last_heartbeat: OffsetDateTime,
}

/// Request to register a new herald.
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterHeraldRequest {
    /// Unique herald identifier.
    pub id: String,
    /// Herald description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Request to update herald heartbeat.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct HeartbeatRequest {
    /// Optional metrics from the herald.
    #[serde(default)]
    pub metrics: HashMap<String, serde_json::Value>,
}

/// Response for heartbeat operation.
#[derive(Debug, Clone, Serialize)]
pub struct HeartbeatResponse {
    /// Current herald status.
    pub status: HeraldStatus,
    /// Last heartbeat timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub last_heartbeat: OffsetDateTime,
}

/// Herald list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeraldsListResponse {
    /// List of heralds.
    pub heralds: Vec<HeraldInfo>,
}

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
    /// Returns list of heralds that changed status.
    pub async fn check_timeouts(&self) -> Vec<(String, HeraldStatus)> {
        let mut changed = Vec::new();
        let mut heralds = self.heralds.write().await;
        let now = OffsetDateTime::now_utc();

        for (id, info) in heralds.iter_mut() {
            let elapsed = (now - info.last_heartbeat).whole_seconds();

            let new_status = if elapsed > 120 {
                HeraldStatus::Disconnected
            } else if elapsed > 60 {
                HeraldStatus::Degraded
            } else {
                continue;
            };

            if info.status != new_status {
                info.status = new_status;
                changed.push((id.clone(), new_status));
            }
        }

        changed
    }
}
