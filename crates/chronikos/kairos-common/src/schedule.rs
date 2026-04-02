//! Schedule types for Kairos time management service.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Unique identifier for a schedule (UUID string).
pub type ScheduleId = String;

/// Schedule priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "low"),
            Priority::Normal => write!(f, "normal"),
            Priority::High => write!(f, "high"),
            Priority::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Priority::Low),
            "normal" | "norm" => Ok(Priority::Normal),
            "high" => Ok(Priority::High),
            "urgent" => Ok(Priority::Urgent),
            _ => Err(format!("Unknown priority: {}", s)),
        }
    }
}

/// Schedule status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleStatus {
    #[default]
    Active,
    Paused,
    Completed,
    Triggered, // Triggered but not yet consumed by kairos-herald
}

impl std::fmt::Display for ScheduleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ScheduleStatus::Active => write!(f, "active"),
            ScheduleStatus::Paused => write!(f, "paused"),
            ScheduleStatus::Completed => write!(f, "completed"),
            ScheduleStatus::Triggered => write!(f, "triggered"),
        }
    }
}

/// Period for recurring schedules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl std::fmt::Display for Period {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Period::Minutely => write!(f, "minutely"),
            Period::Hourly => write!(f, "hourly"),
            Period::Daily => write!(f, "daily"),
            Period::Weekly => write!(f, "weekly"),
            Period::Monthly => write!(f, "monthly"),
            Period::Yearly => write!(f, "yearly"),
        }
    }
}

impl std::str::FromStr for Period {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minutely" | "minute" => Ok(Period::Minutely),
            "hourly" | "hour" => Ok(Period::Hourly),
            "daily" | "day" => Ok(Period::Daily),
            "weekly" | "week" => Ok(Period::Weekly),
            "monthly" | "month" => Ok(Period::Monthly),
            "yearly" | "year" => Ok(Period::Yearly),
            _ => Err(format!("Unknown period: {}", s)),
        }
    }
}

/// Trigger specification for a schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TriggerSpec {
    /// One-time event at a specific time.
    Once {
        #[serde(with = "time::serde::rfc3339")]
        at: OffsetDateTime,
    },
    /// Relative delay from now (in seconds).
    In { duration_seconds: u64 },
    /// Recurring event.
    Every {
        period: Period,
        /// Optional time of day for daily/weekly/monthly/yearly (e.g., "09:00")
        at_time: Option<String>,
    },
    /// Cron expression (v2).
    #[allow(dead_code)]
    Cron { expression: String },
}

/// A scheduled event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    /// Unique schedule identifier (UUID).
    pub id: ScheduleId,
    /// Human-readable name/description.
    pub name: String,
    /// Trigger specification.
    pub trigger: TriggerSpec,
    /// Custom payload to be included in triggered events.
    pub payload: serde_json::Value,
    /// Tags for filtering.
    pub tags: Vec<String>,
    /// Schedule priority.
    pub priority: Priority,
    /// Current status.
    pub status: ScheduleStatus,
    /// Creation timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Next scheduled fire time.
    #[serde(with = "time::serde::rfc3339::option")]
    pub next_fire: Option<OffsetDateTime>,
    /// Last fire time.
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_fire: Option<OffsetDateTime>,
}

/// Request to create a new schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateScheduleRequest {
    /// Human-readable name/description.
    pub name: String,
    /// Trigger specification.
    pub trigger: TriggerSpec,
    /// Custom payload (optional, defaults to empty object).
    #[serde(default)]
    pub payload: serde_json::Value,
    /// Tags for filtering (optional).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Schedule priority (optional, defaults to Normal).
    #[serde(default)]
    pub priority: Priority,
}

/// Request to update a schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateScheduleRequest {
    /// New status (optional).
    pub status: Option<ScheduleStatus>,
    /// Defer next fire time (optional, ISO 8601 duration or RFC3339 timestamp).
    pub defer_until: Option<String>,
}

/// A schedule that has been triggered and is ready to be consumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredSchedule {
    /// The original schedule.
    pub schedule: Schedule,
    /// When it was triggered.
    #[serde(with = "time::serde::rfc3339")]
    pub triggered_at: OffsetDateTime,
}

/// Request to acknowledge triggered schedules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckTriggeredRequest {
    /// Schedule IDs to acknowledge.
    pub ids: Vec<ScheduleId>,
}

/// Schedules list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulesListResponse {
    /// List of schedules.
    pub schedules: Vec<Schedule>,
    /// Total count.
    pub total: usize,
}

/// Service status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    /// Service is healthy.
    pub healthy: bool,
    /// Total number of active schedules.
    pub active_schedules: usize,
    /// Number of triggered schedules waiting to be consumed.
    pub pending_triggered: usize,
    /// Next scheduled fire time.
    #[serde(with = "time::serde::rfc3339::option")]
    pub next_fire: Option<OffsetDateTime>,
}
