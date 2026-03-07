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
            if let Some(next_fire) = schedule.next_fire
                && next_fire <= now
            {
                info!("Schedule {} '{}' is due, marking as triggered", schedule.id, schedule.name);

                // Mark as triggered so kairos-herald can pick it up
                self.store
                    .update_fire_times(&schedule.id, Some(next_fire), Some(next_fire), ScheduleStatus::Triggered)
                    .await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::{Period, TriggerSpec};
    use crate::store::calculate_next_fire;
    use time::ext::NumericalDuration;
    use time::macros::datetime;

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

    // === Monthly Tests ===

    #[test]
    fn test_calculate_next_fire_monthly_normal() {
        let now = datetime!(2025-03-15 10:00 UTC);
        let next = calculate_next_fire(&Period::Monthly, &None, now).unwrap();
        assert_eq!(next.month(), time::Month::April);
        assert_eq!(next.day(), 15);
    }

    #[test]
    fn test_calculate_next_fire_monthly_31_to_28() {
        // Jan 31 -> Feb 28 (non-leap year)
        let now = datetime!(2025-01-31 10:00 UTC);
        let next = calculate_next_fire(&Period::Monthly, &None, now).unwrap();
        assert_eq!(next.month(), time::Month::February);
        assert_eq!(next.day(), 28);
        assert_eq!(next.year(), 2025);
    }

    #[test]
    fn test_calculate_next_fire_monthly_31_to_29_leap_year() {
        // Jan 31 -> Feb 29 (leap year)
        let now = datetime!(2024-01-31 10:00 UTC);
        let next = calculate_next_fire(&Period::Monthly, &None, now).unwrap();
        assert_eq!(next.month(), time::Month::February);
        assert_eq!(next.day(), 29);
        assert_eq!(next.year(), 2024);
    }

    #[test]
    fn test_calculate_next_fire_monthly_december_wrap() {
        // Dec 15 -> Jan 15 next year
        let now = datetime!(2025-12-15 10:00 UTC);
        let next = calculate_next_fire(&Period::Monthly, &None, now).unwrap();
        assert_eq!(next.month(), time::Month::January);
        assert_eq!(next.day(), 15);
        assert_eq!(next.year(), 2026);
    }

    #[test]
    fn test_calculate_next_fire_monthly_30_day_month() {
        // Mar 31 -> Apr 30 (April has only 30 days)
        let now = datetime!(2025-03-31 10:00 UTC);
        let next = calculate_next_fire(&Period::Monthly, &None, now).unwrap();
        assert_eq!(next.month(), time::Month::April);
        assert_eq!(next.day(), 30);
    }

    #[test]
    fn test_calculate_next_fire_monthly_feb_28_to_march() {
        // Feb 28 -> Mar 28
        let now = datetime!(2025-02-28 10:00 UTC);
        let next = calculate_next_fire(&Period::Monthly, &None, now).unwrap();
        assert_eq!(next.month(), time::Month::March);
        assert_eq!(next.day(), 28);
    }

    #[test]
    fn test_calculate_next_fire_monthly_preserves_time() {
        let now = datetime!(2025-03-15 14:30:45 UTC);
        let next = calculate_next_fire(&Period::Monthly, &None, now).unwrap();
        assert_eq!(next.hour(), 14);
        assert_eq!(next.minute(), 30);
        assert_eq!(next.second(), 45);
    }

    // === Yearly Tests ===

    #[test]
    fn test_calculate_next_fire_yearly_normal() {
        let now = datetime!(2025-03-15 10:00 UTC);
        let next = calculate_next_fire(&Period::Yearly, &None, now).unwrap();
        assert_eq!(next.year(), 2026);
        assert_eq!(next.month(), time::Month::March);
        assert_eq!(next.day(), 15);
    }

    #[test]
    fn test_calculate_next_fire_yearly_leap_to_non_leap() {
        // Feb 29 (leap year) -> Feb 28 (non-leap year)
        let now = datetime!(2024-02-29 10:00 UTC);
        let next = calculate_next_fire(&Period::Yearly, &None, now).unwrap();
        assert_eq!(next.year(), 2025);
        assert_eq!(next.month(), time::Month::February);
        assert_eq!(next.day(), 28);
    }

    #[test]
    fn test_calculate_next_fire_yearly_leap_to_leap() {
        // Feb 29 schedule downgrades to Feb 28 and stays there
        // (we don't upgrade back to Feb 29 to avoid affecting explicit Feb 28 schedules)
        let now = datetime!(2024-02-29 10:00 UTC);
        let next1 = calculate_next_fire(&Period::Yearly, &None, now).unwrap();
        assert_eq!(next1.day(), 28); // 2025
        assert_eq!(next1.year(), 2025);
        let next2 = calculate_next_fire(&Period::Yearly, &None, next1).unwrap();
        assert_eq!(next2.day(), 28); // 2026
        let next3 = calculate_next_fire(&Period::Yearly, &None, next2).unwrap();
        assert_eq!(next3.day(), 28); // 2027
        let next4 = calculate_next_fire(&Period::Yearly, &None, next3).unwrap();
        assert_eq!(next4.year(), 2028);
        assert_eq!(next4.day(), 28); // Stays on Feb 28 even in leap year
    }

    #[test]
    fn test_calculate_next_fire_yearly_explicit_feb_28() {
        // Explicit Feb 28 schedule should stay on Feb 28, never upgrade to Feb 29
        let now = datetime!(2025-02-28 10:00 UTC);
        let next1 = calculate_next_fire(&Period::Yearly, &None, now).unwrap();
        assert_eq!(next1.day(), 28);
        assert_eq!(next1.year(), 2026);
        // Transition to leap year 2028
        let next2 = calculate_next_fire(&Period::Yearly, &None, next1).unwrap();
        assert_eq!(next2.day(), 28); // 2027
        let next3 = calculate_next_fire(&Period::Yearly, &None, next2).unwrap();
        assert_eq!(next3.year(), 2028);
        assert_eq!(next3.day(), 28); // Still Feb 28, not upgraded to Feb 29
    }

    #[test]
    fn test_calculate_next_fire_yearly_preserves_time() {
        let now = datetime!(2025-06-15 08:45:30 UTC);
        let next = calculate_next_fire(&Period::Yearly, &None, now).unwrap();
        assert_eq!(next.hour(), 8);
        assert_eq!(next.minute(), 45);
        assert_eq!(next.second(), 30);
    }

    #[test]
    fn test_calculate_next_fire_yearly_dec_31() {
        let now = datetime!(2025-12-31 23:59:59 UTC);
        let next = calculate_next_fire(&Period::Yearly, &None, now).unwrap();
        assert_eq!(next.year(), 2026);
        assert_eq!(next.month(), time::Month::December);
        assert_eq!(next.day(), 31);
    }
}
