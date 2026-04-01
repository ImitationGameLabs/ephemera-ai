//! Herald types for Agora event hub.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Herald status based on heartbeat health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HeraldStatus {
    /// Herald is active and sending heartbeats.
    #[default]
    Active,
    /// Herald has not sent heartbeat within timeout threshold.
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
