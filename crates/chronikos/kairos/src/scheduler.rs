//! Scheduler engine for Kairos.

use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{debug, error, info};

use crate::schedule::{ScheduleStatus, TriggerSpec};
use crate::store::{calculate_initial_next_fire, calculate_next_fire, ScheduleStore};

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

    /// Performs a scheduler tick at the current time.
    async fn tick(&self) -> anyhow::Result<()> {
        self.tick_at(OffsetDateTime::now_utc()).await
    }

    /// Performs a scheduler tick at a specific point in time.
    ///
    /// This is the core scheduling logic, parameterized for deterministic testing.
    /// When a schedule is triggered:
    /// - Recurring schedules: `next_fire` is immediately calculated to the next occurrence
    /// - One-time schedules: `next_fire` is set to `None`
    ///
    /// This prevents double-triggering even if status is manually reset to Active.
    pub async fn tick_at(&self, now: OffsetDateTime) -> anyhow::Result<()> {
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

                // Calculate new next_fire immediately:
                // - For recurring: next occurrence
                // - For one-time: None
                let new_next_fire = match &schedule.trigger {
                    TriggerSpec::Every { period, at_time } => {
                        Some(calculate_next_fire(period, at_time, now)?)
                    }
                    // Once, In are one-time triggers; Cron TODO: should be recurring when implemented
                    _ => None,
                };

                // Mark as triggered with updated next_fire
                self.store
                    .update_fire_times(&schedule.id, new_next_fire, Some(next_fire), ScheduleStatus::Triggered)
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

    // === Bug 1 Tests: Recurring schedule next_fire updated on trigger ===

    #[tokio::test]
    async fn test_recurring_schedule_next_fire_updated_on_trigger() {
        use crate::schedule::{Priority, Schedule};

        let store = Arc::new(ScheduleStore::new(":memory:").await.unwrap());
        let scheduler = Scheduler::new(store.clone(), 1000);

        // Create hourly schedule
        let schedule = Schedule {
            id: "hourly".into(),
            name: "Hourly test".into(),
            trigger: TriggerSpec::Every { period: Period::Hourly, at_time: None },
            payload: serde_json::json!({}),
            tags: vec![],
            priority: Priority::Normal,
            status: ScheduleStatus::Active,
            created_at: datetime!(2025-03-12 08:00 UTC),
            next_fire: Some(datetime!(2025-03-12 09:00 UTC)),
            last_fire: None,
        };
        store.create(&schedule).await.unwrap();

        // Trigger at 09:00:05
        scheduler.tick_at(datetime!(2025-03-12 09:00:05 UTC)).await.unwrap();

        let triggered = store.get("hourly").await.unwrap().unwrap();
        assert_eq!(triggered.status, ScheduleStatus::Triggered);

        // Key assertion: next_fire should be future time, not old 09:00
        assert!(
            triggered.next_fire.unwrap() > datetime!(2025-03-12 09:00:05 UTC),
            "next_fire should be updated to future time, got {:?}",
            triggered.next_fire
        );
    }

    #[tokio::test]
    async fn test_no_double_trigger_after_manual_status_reset() {
        use crate::schedule::{Priority, Schedule};

        let store = Arc::new(ScheduleStore::new(":memory:").await.unwrap());
        let scheduler = Scheduler::new(store.clone(), 1000);

        let schedule = Schedule {
            id: "hourly".into(),
            name: "Hourly test".into(),
            trigger: TriggerSpec::Every { period: Period::Hourly, at_time: None },
            payload: serde_json::json!({}),
            tags: vec![],
            priority: Priority::Normal,
            status: ScheduleStatus::Active,
            created_at: datetime!(2025-03-12 08:00 UTC),
            next_fire: Some(datetime!(2025-03-12 09:00 UTC)),
            last_fire: None,
        };
        store.create(&schedule).await.unwrap();

        // First trigger
        scheduler.tick_at(datetime!(2025-03-12 09:00:05 UTC)).await.unwrap();

        // Simulate error: manually change status back to Active (without updating next_fire)
        store.update_status("hourly", ScheduleStatus::Active).await.unwrap();

        // Second tick: should NOT re-trigger because next_fire is already future
        scheduler.tick_at(datetime!(2025-03-12 09:00:10 UTC)).await.unwrap();

        let result = store.get("hourly").await.unwrap().unwrap();
        // If bug not fixed, status would be Triggered
        // After fix, status stays Active (because next_fire > now)
        assert_eq!(result.status, ScheduleStatus::Active);
    }

    #[tokio::test]
    async fn test_once_schedule_next_fire_cleared_on_trigger() {
        use crate::schedule::{Priority, Schedule};

        let store = Arc::new(ScheduleStore::new(":memory:").await.unwrap());
        let scheduler = Scheduler::new(store.clone(), 1000);

        let schedule = Schedule {
            id: "once".into(),
            name: "One-time test".into(),
            trigger: TriggerSpec::Once { at: datetime!(2025-03-12 09:00 UTC) },
            payload: serde_json::json!({}),
            tags: vec![],
            priority: Priority::Normal,
            status: ScheduleStatus::Active,
            created_at: datetime!(2025-03-12 08:00 UTC),
            next_fire: Some(datetime!(2025-03-12 09:00 UTC)),
            last_fire: None,
        };
        store.create(&schedule).await.unwrap();

        scheduler.tick_at(datetime!(2025-03-12 09:00:05 UTC)).await.unwrap();

        let triggered = store.get("once").await.unwrap().unwrap();
        assert_eq!(triggered.status, ScheduleStatus::Triggered);
        // One-time schedule's next_fire should be cleared
        assert!(triggered.next_fire.is_none());
    }

    #[tokio::test]
    async fn test_full_recurring_schedule_lifecycle() {
        use crate::schedule::{Priority, Schedule};

        let store = Arc::new(ScheduleStore::new(":memory:").await.unwrap());
        let scheduler = Scheduler::new(store.clone(), 1000);

        // Create minutely schedule (simplified for testing)
        let schedule = Schedule {
            id: "minutely".into(),
            name: "Minutely test".into(),
            trigger: TriggerSpec::Every { period: Period::Minutely, at_time: None },
            payload: serde_json::json!({}),
            tags: vec![],
            priority: Priority::Normal,
            status: ScheduleStatus::Active,
            created_at: datetime!(2025-03-12 09:00 UTC),
            next_fire: Some(datetime!(2025-03-12 09:00 UTC)),
            last_fire: None,
        };
        store.create(&schedule).await.unwrap();

        // 09:00:05 trigger -> next_fire immediately updated to 09:01:05
        scheduler.tick_at(datetime!(2025-03-12 09:00:05 UTC)).await.unwrap();
        let after_trigger = store.get("minutely").await.unwrap().unwrap();
        assert_eq!(after_trigger.status, ScheduleStatus::Triggered);
        assert_eq!(after_trigger.next_fire, Some(datetime!(2025-03-12 09:01:05 UTC)));

        // 09:00:30 Ack -> status restored to Active, next_fire unchanged
        store.ack_triggered_at(&["minutely".into()], datetime!(2025-03-12 09:00:30 UTC)).await.unwrap();
        let after_ack = store.get("minutely").await.unwrap().unwrap();
        assert_eq!(after_ack.status, ScheduleStatus::Active);
        assert_eq!(after_ack.next_fire, Some(datetime!(2025-03-12 09:01:05 UTC)));

        // 09:01:00 should NOT trigger (next_fire = 09:01:05 > 09:01:00)
        scheduler.tick_at(datetime!(2025-03-12 09:01:00 UTC)).await.unwrap();
        let before_second = store.get("minutely").await.unwrap().unwrap();
        assert_eq!(before_second.status, ScheduleStatus::Active);

        // 09:01:05 second trigger -> next_fire updated to 09:02:05
        scheduler.tick_at(datetime!(2025-03-12 09:01:05 UTC)).await.unwrap();
        let second_trigger = store.get("minutely").await.unwrap().unwrap();
        assert_eq!(second_trigger.status, ScheduleStatus::Triggered);
        assert_eq!(second_trigger.next_fire, Some(datetime!(2025-03-12 09:02:05 UTC)));
    }
}
