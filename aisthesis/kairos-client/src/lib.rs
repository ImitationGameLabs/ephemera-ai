//! Kairos client library for Ephemera AI.
//!
//! This client provides a convenient interface for interacting with
//! the Kairos time management service.

mod client;

pub use client::{KairosClient, KairosClientError};

// Re-export commonly used types from kairos
pub use kairos::schedule::{
    AckTriggeredRequest, CreateScheduleRequest, Period, Priority, Schedule, ScheduleId,
    ScheduleStatus, SchedulesListResponse, StatusResponse, TriggerSpec, TriggeredSchedule,
    UpdateScheduleRequest,
};
