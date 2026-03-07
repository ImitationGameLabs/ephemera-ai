//! SQLite persistence for pending events.

use crate::event::{CreateEventRequest, Event, EventId, EventPriority, EventStatus};
use anyhow::Result;
use sqlx::{Row, SqlitePool};
use time::OffsetDateTime;

/// SQLite-backed pending event storage.
#[derive(Clone)]
pub struct SqliteEventStore {
    pool: SqlitePool,
}

impl SqliteEventStore {
    pub async fn new(database_path: &str) -> Result<Self> {
        let db_url = format!("sqlite:{}?mode=rwc", database_path);
        let pool = SqlitePool::connect(&db_url).await?;
        Self::run_migrations(&pool).await?;
        Ok(Self { pool })
    }

    async fn run_migrations(pool: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                herald_id TEXT NOT NULL,
                payload TEXT NOT NULL,
                priority TEXT NOT NULL DEFAULT 'normal',
                timestamp TEXT NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)")
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Insert new event. Returns the created event with ID.
    pub async fn insert(&self, req: CreateEventRequest) -> Result<Event> {
        let timestamp = req.timestamp.format(&time::format_description::well_known::Rfc3339)?;

        let result = sqlx::query(
            r#"
            INSERT INTO events (event_type, herald_id, payload, priority, timestamp)
            VALUES (?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(&req.event_type)
        .bind(&req.herald_id)
        .bind(req.payload.to_string())
        .bind(req.priority.to_string())
        .bind(&timestamp)
        .fetch_one(&self.pool)
        .await?;

        let id: i64 = result.get("id");

        Ok(Event {
            id: id as u64,
            event_type: req.event_type,
            herald_id: req.herald_id,
            payload: req.payload,
            priority: req.priority,
            timestamp: req.timestamp,
            status: EventStatus::Pending,
        })
    }

    /// Load all pending events (for startup).
    pub async fn load_all(&self) -> Result<Vec<Event>> {
        let rows = sqlx::query("SELECT * FROM events ORDER BY id")
            .fetch_all(&self.pool)
            .await?;

        rows.into_iter().map(row_to_event).collect()
    }

    /// Delete event by ID. Returns true if deleted.
    pub async fn delete(&self, id: EventId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM events WHERE id = ?")
            .bind(id as i64)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

fn row_to_event(row: sqlx::sqlite::SqliteRow) -> Result<Event> {
    let timestamp_str: String = row.get("timestamp");
    let timestamp =
        OffsetDateTime::parse(&timestamp_str, &time::format_description::well_known::Rfc3339)?;

    let payload_str: String = row.get("payload");
    let payload = serde_json::from_str(&payload_str)?;

    let priority_str: String = row.get("priority");
    let priority = parse_priority(&priority_str);

    Ok(Event {
        id: row.get::<i64, _>("id") as u64,
        event_type: row.get("event_type"),
        herald_id: row.get("herald_id"),
        payload,
        priority,
        timestamp,
        status: EventStatus::Pending,
    })
}

fn parse_priority(s: &str) -> EventPriority {
    match s {
        "low" => EventPriority::Low,
        "high" => EventPriority::High,
        "urgent" => EventPriority::Urgent,
        _ => EventPriority::Normal,
    }
}
