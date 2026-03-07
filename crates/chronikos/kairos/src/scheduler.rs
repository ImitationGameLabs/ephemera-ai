//! Scheduler engine for Kairos.

use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{debug, error, info};

use crate::schedule::ScheduleStatus;
use crate::store::{calculate_initial_next_fire, ScheduleStore};

/// Scheduler engine that checks for due schedules.
pub struct Scheduler {
    store: Arc<ScheduleStore>,
    tick_interval: Duration,
}

impl Scheduler {
    /// Creates a new scheduler.
    pub fn new(store: Arc<ScheduleStore>, tick_interval_ms: u64) -> Self {
        Self {
            store,
            tick_interval: Duration::from_millis(tick_interval_ms),
        }
    }

    /// Starts the scheduler loop.
    pub async fn run(self) {
        let mut interval = tokio::time::interval(self.tick_interval);

        info!("Scheduler started with tick interval {:?}", self.tick_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.tick().await {
                error!("Scheduler tick error: {}", e);
            }
        }
    }

    async fn tick(&self) -> anyhow::Result<()> {
        let now = OffsetDateTime::now_utc();
        debug!("Scheduler tick at {}", now);

        // Get all active schedules
        let schedules = self.store.list(Some(ScheduleStatus::Active), None).await?;

        for mut schedule in schedules {
            // Calculate next_fire if not set
            if schedule.next_fire.is_none() {
                let next = calculate_initial_next_fire(&schedule.trigger, schedule.created_at)?;
                schedule.next_fire = Some(next);
                self.store
                    .update_fire_times(&schedule.id, Some(next), None, ScheduleStatus::Active)
                    .await?;
                debug!("Calculated initial next_fire for schedule {}: {:?}", schedule.id, next);
            }

            // Check if schedule is due
            if let Some(next_fire) = schedule.next_fire {
                if next_fire <= now {
                    info!("Schedule {} '{}' is due, marking as triggered", schedule.id, schedule.name);

                    // Mark as triggered so kairos-herald can pick it up
                    self.store
                        .update_fire_times(&schedule.id, Some(next_fire), Some(next_fire), ScheduleStatus::Triggered)
                        .await?;
                }
            }
        }

        Ok(())
    }
}

/// Initializes next_fire times for schedules that don't have one.
pub async fn initialize_schedule(store: &ScheduleStore, schedule_id: &str) -> anyhow::Result<()> {
    let schedule = store
        .get(schedule_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Schedule not found: {}", schedule_id))?;

    if schedule.next_fire.is_some() {
        return Ok(());
    }

    let next = calculate_initial_next_fire(&schedule.trigger, schedule.created_at)?;
    store
        .update_fire_times(schedule_id, Some(next), None, ScheduleStatus::Active)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{Period, TriggerSpec};
    use crate::store::calculate_next_fire;
    use time::ext::NumericalDuration;

    #[test]
    fn test_calculate_next_fire_daily() {
        let now = OffsetDateTime::now_utc();
        let at_time = Some("09:00".to_string());

        let next = calculate_next_fire(&Period::Daily, &at_time, now).unwrap();

        // Should be tomorrow at 09:00
        assert!(next > now);
        assert_eq!(next.hour(), 9);
        assert_eq!(next.minute(), 0);
    }

    #[test]
    fn test_calculate_next_fire_hourly() {
        let now = OffsetDateTime::now_utc();

        let next = calculate_next_fire(&Period::Hourly, &None, now).unwrap();

        // Should be about 1 hour from now
        assert!(next > now);
        assert!(next - now <= 2.hours());
    }

    #[test]
    fn test_calculate_initial_next_fire_once() {
        let now = OffsetDateTime::now_utc();
        let at = now + 1.hours();
        let trigger = TriggerSpec::Once { at };

        let next = calculate_initial_next_fire(&trigger, now).unwrap();

        assert_eq!(next, at);
    }

    #[test]
    fn test_calculate_initial_next_fire_in() {
        let now = OffsetDateTime::now_utc();
        let trigger = TriggerSpec::In {
            duration_seconds: 3600,
        };

        let next = calculate_initial_next_fire(&trigger, now).unwrap();

        assert!(next > now);
        assert!(next - now <= 2.hours());
    }
}
