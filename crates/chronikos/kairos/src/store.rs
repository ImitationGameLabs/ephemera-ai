//! SQLite storage layer for Kairos schedules.

use anyhow::Result;
use sqlx::SqlitePool;
use time::OffsetDateTime;

use crate::schedule::*;

/// SQLite-based schedule store.
pub struct ScheduleStore {
    pool: SqlitePool,
}

impl ScheduleStore {
    /// Creates a new store with the given database path.
    pub async fn new(database_path: &str) -> Result<Self> {
        let db_url = format!("sqlite:{}?mode=rwc", database_path);
        let pool = SqlitePool::connect(&db_url).await?;

        // Run migrations
        Self::run_migrations(&pool).await?;

        Ok(Self { pool })
    }

    async fn run_migrations(pool: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schedules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                trigger_type TEXT NOT NULL,
                trigger_at TEXT,
                trigger_duration_seconds INTEGER,
                trigger_period TEXT,
                trigger_at_time TEXT,
                trigger_cron_expression TEXT,
                payload TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '[]',
                priority TEXT NOT NULL DEFAULT 'normal',
                status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL,
                next_fire TEXT,
                last_fire TEXT
            )
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Creates a new schedule.
    pub async fn create(&self, schedule: &Schedule) -> Result<()> {
        let (trigger_type, trigger_at, trigger_duration_seconds, trigger_period, trigger_at_time, trigger_cron_expression) =
            self.serialize_trigger(&schedule.trigger);

        sqlx::query(
            r#"
            INSERT INTO schedules (
                id, name, trigger_type, trigger_at, trigger_duration_seconds,
                trigger_period, trigger_at_time, trigger_cron_expression,
                payload, tags, priority, status, created_at, next_fire, last_fire
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&schedule.id)
        .bind(&schedule.name)
        .bind(&trigger_type)
        .bind(trigger_at.as_ref())
        .bind(trigger_duration_seconds)
        .bind(trigger_period.as_ref())
        .bind(trigger_at_time.as_ref())
        .bind(trigger_cron_expression.as_ref())
        .bind(serde_json::to_string(&schedule.payload)?)
        .bind(serde_json::to_string(&schedule.tags)?)
        .bind(schedule.priority.to_string())
        .bind(schedule.status.to_string())
        .bind(schedule.created_at.format(&time::format_description::well_known::Rfc3339)?)
        .bind(schedule.next_fire.map(|t| t.format(&time::format_description::well_known::Rfc3339)).transpose()?)
        .bind(schedule.last_fire.map(|t| t.format(&time::format_description::well_known::Rfc3339)).transpose()?)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gets a schedule by ID.
    pub async fn get(&self, id: &str) -> Result<Option<Schedule>> {
        let row = sqlx::query("SELECT * FROM schedules WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(self.deserialize_schedule(row)?)),
            None => Ok(None),
        }
    }

    /// Lists schedules with optional filtering.
    pub async fn list(&self, status: Option<ScheduleStatus>, tag: Option<&str>) -> Result<Vec<Schedule>> {
        let mut query = String::from("SELECT * FROM schedules WHERE 1=1");

        if status.is_some() {
            query.push_str(" AND status = ?");
        }
        if tag.is_some() {
            query.push_str(" AND tags LIKE ?");
        }

        let mut q = sqlx::query(&query);

        if let Some(s) = status {
            q = q.bind(s.to_string());
        }
        if let Some(t) = tag {
            q = q.bind(format!("%\"{}\"", t));
        }

        let rows = q.fetch_all(&self.pool).await?;

        let mut schedules = Vec::new();
        for row in rows {
            schedules.push(self.deserialize_schedule(row)?);
        }

        Ok(schedules)
    }

    /// Gets the next schedule to fire.
    pub async fn get_next(&self) -> Result<Option<Schedule>> {
        let now = OffsetDateTime::now_utc();
        let now_str = now.format(&time::format_description::well_known::Rfc3339)?;

        let row = sqlx::query(
            r#"
            SELECT * FROM schedules
            WHERE status = 'active' AND next_fire IS NOT NULL AND next_fire <= ?
            ORDER BY next_fire ASC
            LIMIT 1
            "#,
        )
        .bind(&now_str)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(self.deserialize_schedule(row)?)),
            None => Ok(None),
        }
    }

    /// Gets all triggered schedules (ready to be consumed by herald).
    pub async fn get_triggered(&self) -> Result<Vec<TriggeredSchedule>> {
        let rows = sqlx::query(
            r#"
            SELECT * FROM schedules
            WHERE status = 'triggered'
            ORDER BY next_fire ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut triggered = Vec::new();
        for row in rows {
            let schedule = self.deserialize_schedule(row)?;
            let triggered_at = schedule.next_fire.unwrap_or(schedule.created_at);
            triggered.push(TriggeredSchedule { schedule, triggered_at });
        }

        Ok(triggered)
    }

    /// Updates a schedule's status.
    pub async fn update_status(&self, id: &str, status: ScheduleStatus) -> Result<()> {
        sqlx::query("UPDATE schedules SET status = ? WHERE id = ?")
            .bind(status.to_string())
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Updates next_fire and last_fire times.
    pub async fn update_fire_times(
        &self,
        id: &str,
        next_fire: Option<OffsetDateTime>,
        last_fire: Option<OffsetDateTime>,
        status: ScheduleStatus,
    ) -> Result<()> {
        let next_str = next_fire.map(|t| t.format(&time::format_description::well_known::Rfc3339)).transpose()?;
        let last_str = last_fire.map(|t| t.format(&time::format_description::well_known::Rfc3339)).transpose()?;

        sqlx::query(
            "UPDATE schedules SET next_fire = ?, last_fire = ?, status = ? WHERE id = ?",
        )
        .bind(next_str)
        .bind(last_str)
        .bind(status.to_string())
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes a schedule.
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM schedules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Acknowledges triggered schedules (marks them as completed or reschedules).
    pub async fn ack_triggered(&self, ids: &[ScheduleId]) -> Result<usize> {
        let mut count = 0;
        for id in ids {
            if let Some(schedule) = self.get(id).await? {
                if schedule.status != ScheduleStatus::Triggered {
                    continue;
                }

                // For recurring schedules, calculate next fire time and reactivate
                if let TriggerSpec::Every { period, at_time } = &schedule.trigger {
                    let next = calculate_next_fire(period, at_time, OffsetDateTime::now_utc())?;
                    self.update_fire_times(id, Some(next), schedule.next_fire, ScheduleStatus::Active)
                        .await?;
                } else {
                    // One-time schedules are completed
                    self.update_status(id, ScheduleStatus::Completed).await?;
                }
                count += 1;
            }
        }
        Ok(count)
    }

    /// Gets service statistics.
    pub async fn get_stats(&self) -> Result<(usize, usize, Option<OffsetDateTime>)> {
        let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM schedules WHERE status = 'active'")
            .fetch_one(&self.pool)
            .await?;

        let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM schedules WHERE status = 'triggered'")
            .fetch_one(&self.pool)
            .await?;

        let next_fire: Option<String> = sqlx::query_scalar(
            "SELECT MIN(next_fire) FROM schedules WHERE status = 'active' AND next_fire IS NOT NULL",
        )
        .fetch_optional(&self.pool)
        .await?;

        let next_fire_time = match next_fire {
            Some(s) => Some(OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)?),
            None => None,
        };

        Ok((active as usize, pending as usize, next_fire_time))
    }

    fn serialize_trigger(&self, trigger: &TriggerSpec) -> (String, Option<String>, Option<i64>, Option<String>, Option<String>, Option<String>) {
        match trigger {
            TriggerSpec::Once { at } => (
                "once".to_string(),
                Some(at.format(&time::format_description::well_known::Rfc3339).unwrap()),
                None,
                None,
                None,
                None,
            ),
            TriggerSpec::In { duration_seconds } => (
                "in".to_string(),
                None,
                Some(*duration_seconds as i64),
                None,
                None,
                None,
            ),
            TriggerSpec::Every { period, at_time } => (
                "every".to_string(),
                None,
                None,
                Some(period.to_string()),
                at_time.clone(),
                None,
            ),
            TriggerSpec::Cron { expression } => (
                "cron".to_string(),
                None,
                None,
                None,
                None,
                Some(expression.clone()),
            ),
        }
    }

    fn deserialize_schedule(&self, row: sqlx::sqlite::SqliteRow) -> Result<Schedule> {
        use sqlx::Row;

        let id: String = row.get("id");
        let name: String = row.get("name");
        let trigger_type: String = row.get("trigger_type");
        let trigger_at: Option<String> = row.get("trigger_at");
        let trigger_duration_seconds: Option<i64> = row.get("trigger_duration_seconds");
        let trigger_period: Option<String> = row.get("trigger_period");
        let trigger_at_time: Option<String> = row.get("trigger_at_time");
        let trigger_cron_expression: Option<String> = row.get("trigger_cron_expression");
        let payload_str: String = row.get("payload");
        let tags_str: String = row.get("tags");
        let priority_str: String = row.get("priority");
        let status_str: String = row.get("status");
        let created_at_str: String = row.get("created_at");
        let next_fire_str: Option<String> = row.get("next_fire");
        let last_fire_str: Option<String> = row.get("last_fire");

        let trigger = match trigger_type.as_str() {
            "once" => TriggerSpec::Once {
                at: OffsetDateTime::parse(
                    &trigger_at.unwrap(),
                    &time::format_description::well_known::Rfc3339,
                )?,
            },
            "in" => TriggerSpec::In {
                duration_seconds: trigger_duration_seconds.unwrap() as u64,
            },
            "every" => TriggerSpec::Every {
                period: trigger_period.unwrap().parse().map_err(|e: String| anyhow::anyhow!("{}", e))?,
                at_time: trigger_at_time,
            },
            "cron" => TriggerSpec::Cron {
                expression: trigger_cron_expression.unwrap(),
            },
            _ => return Err(anyhow::anyhow!("Unknown trigger type: {}", trigger_type)),
        };

        let payload: serde_json::Value = serde_json::from_str(&payload_str)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str)?;
        let priority: Priority = parse_priority(&priority_str)?;
        let status: ScheduleStatus = parse_status(&status_str)?;
        let created_at = OffsetDateTime::parse(&created_at_str, &time::format_description::well_known::Rfc3339)?;
        let next_fire = next_fire_str
            .map(|s| OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339))
            .transpose()?;
        let last_fire = last_fire_str
            .map(|s| OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339))
            .transpose()?;

        Ok(Schedule {
            id,
            name,
            trigger,
            payload,
            tags,
            priority,
            status,
            created_at,
            next_fire,
            last_fire,
        })
    }
}

fn parse_priority(s: &str) -> Result<Priority> {
    match s {
        "low" => Ok(Priority::Low),
        "normal" => Ok(Priority::Normal),
        "high" => Ok(Priority::High),
        "urgent" => Ok(Priority::Urgent),
        _ => Err(anyhow::anyhow!("Unknown priority: {}", s)),
    }
}

fn parse_status(s: &str) -> Result<ScheduleStatus> {
    match s {
        "active" => Ok(ScheduleStatus::Active),
        "paused" => Ok(ScheduleStatus::Paused),
        "completed" => Ok(ScheduleStatus::Completed),
        "triggered" => Ok(ScheduleStatus::Triggered),
        _ => Err(anyhow::anyhow!("Unknown status: {}", s)),
    }
}

/// Calculates the next fire time for a recurring schedule.
pub fn calculate_next_fire(
    period: &Period,
    at_time: &Option<String>,
    from: OffsetDateTime,
) -> Result<OffsetDateTime> {
    // Parse at_time if provided (e.g., "09:00")
    let (hour, minute) = if let Some(time_str) = at_time {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid at_time format: {}", time_str));
        }
        (
            parts[0].parse::<u8>()?,
            parts[1].parse::<u8>()?,
        )
    } else {
        (from.hour(), from.minute())
    };

    let next = match period {
        Period::Minutely => {
            // Next minute
            from + time::Duration::minutes(1)
        }
        Period::Hourly => {
            // Next hour at the same minute
            from + time::Duration::hours(1)
        }
        Period::Daily => {
            // Next day at the specified time
            let candidate = from.replace_hour(hour)?.replace_minute(minute)?.replace_second(0)?;
            if candidate > from {
                candidate
            } else {
                candidate + time::Duration::days(1)
            }
        }
        Period::Weekly => {
            // Next week at the specified time
            let candidate = from.replace_hour(hour)?.replace_minute(minute)?.replace_second(0)?;
            if candidate > from {
                candidate
            } else {
                candidate + time::Duration::weeks(1)
            }
        }
        Period::Monthly => {
            // Next month at the specified time (same day)
            let candidate = from.replace_hour(hour)?.replace_minute(minute)?.replace_second(0)?;
            if candidate > from {
                candidate
            } else {
                // Add approximately 30 days (simplified)
                candidate + time::Duration::days(30)
            }
        }
        Period::Yearly => {
            // Next year at the specified time
            let candidate = from.replace_hour(hour)?.replace_minute(minute)?.replace_second(0)?;
            if candidate > from {
                candidate
            } else {
                candidate + time::Duration::days(365)
            }
        }
    };

    Ok(next)
}

/// Calculates the initial next_fire time for a new schedule.
pub fn calculate_initial_next_fire(trigger: &TriggerSpec, now: OffsetDateTime) -> Result<OffsetDateTime> {
    match trigger {
        TriggerSpec::Once { at } => Ok(*at),
        TriggerSpec::In { duration_seconds } => {
            Ok(now + time::Duration::seconds(*duration_seconds as i64))
        }
        TriggerSpec::Every { period, at_time } => {
            calculate_next_fire(period, at_time, now)
        }
        TriggerSpec::Cron { .. } => {
            // v2: implement cron parsing
            Err(anyhow::anyhow!("Cron expressions not yet supported"))
        }
    }
}
