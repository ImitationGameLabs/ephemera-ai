use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use kairos_client::{CreateScheduleRequest, KairosClient, Period, Priority, Schedule, ScheduleStatus, TriggerSpec};
use std::env;
use time::{format_description, OffsetDateTime};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const ENV_KAIROS_URL: &str = "KAIROS_URL";
const DEFAULT_URL: &str = "http://localhost:8081";

fn get_server_url() -> String {
    env::var(ENV_KAIROS_URL)
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_URL.to_string())
}

fn get_client() -> KairosClient {
    let url = get_server_url();
    KairosClient::new(url)
}

#[derive(Parser)]
#[command(name = "kairos-cli")]
#[command(about = "CLI client for Kairos time management service - the brain of time")]
#[command(version)]
struct Cli {
    /// Kairos server URL (overrides KAIROS_URL env var)
    #[arg(short, long, global = true)]
    url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a schedule with unified interface
    Schedule {
        /// Schedule name/description
        name: String,
        /// When to trigger (RFC3339 timestamp or relative time like +1h, +30m; relative units are lowercase)
        #[arg(long)]
        when: String,
        /// Repeat period (minutely, hourly, daily, weekly, monthly, yearly)
        #[arg(long)]
        repeat: Option<Period>,
        /// JSON payload to include in triggered events
        #[arg(long)]
        payload: Option<String>,
        /// Comma-separated tags
        #[arg(long)]
        tags: Option<String>,
        /// Priority (low, normal, high, urgent)
        #[arg(long, default_value = "normal")]
        priority: Priority,
    },
    /// Schedule a one-time event at a specific time
    At {
        /// RFC3339 timestamp (e.g., 2026-03-15T14:30:00Z)
        time: String,
        /// Schedule name/description
        name: String,
        /// JSON payload
        #[arg(long)]
        payload: Option<String>,
        /// Priority
        #[arg(long, default_value = "normal")]
        priority: Priority,
    },
    /// Schedule an event after a delay
    In {
        /// Duration (e.g., 30s, 5m, 2h, 1d; units must be lowercase)
        duration: String,
        /// Schedule name/description
        name: String,
        /// JSON payload
        #[arg(long)]
        payload: Option<String>,
        /// Priority
        #[arg(long, default_value = "normal")]
        priority: Priority,
    },
    /// Schedule a recurring event
    Every {
        /// Period (minutely, hourly, daily, weekly, monthly, yearly)
        period: Period,
        /// Schedule name/description
        name: String,
        /// Time of day for daily/weekly/etc (e.g., "09:00")
        #[arg(long)]
        at: Option<String>,
        /// JSON payload
        #[arg(long)]
        payload: Option<String>,
        /// Priority
        #[arg(long, default_value = "normal")]
        priority: Priority,
    },
    /// List schedules
    List {
        /// Filter by status (active, paused, completed, triggered; case-sensitive)
        #[arg(long)]
        status: Option<String>,
        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },
    /// Show the next upcoming schedule
    Next,
    /// Cancel a schedule
    Cancel {
        /// Schedule ID
        id: String,
    },
    /// Show service status
    Status,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "kairos-cli=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    let client = get_client();

    let result = match cli.command {
        Commands::Schedule {
            name,
            when,
            repeat,
            payload,
            tags,
            priority,
        } => handle_schedule(name, when, repeat, payload, tags, priority, &client).await,
        Commands::At {
            time,
            name,
            payload,
            priority,
        } => handle_at(time, name, payload, priority, &client).await,
        Commands::In {
            duration,
            name,
            payload,
            priority,
        } => handle_in(duration, name, payload, priority, &client).await,
        Commands::Every {
            period,
            name,
            at,
            payload,
            priority,
        } => handle_every(period, name, at, payload, priority, &client).await,
        Commands::List { status, tag } => handle_list(status, tag, &client).await,
        Commands::Next => handle_next(&client).await,
        Commands::Cancel { id } => handle_cancel(id, &client).await,
        Commands::Status => handle_status(&client).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

// === Command Handlers ===

async fn handle_schedule(
    name: String,
    when: String,
    repeat: Option<Period>,
    payload: Option<String>,
    tags: Option<String>,
    priority: Priority,
    client: &KairosClient,
) -> Result<()> {
    let trigger = if let Some(period) = repeat {
        TriggerSpec::Every {
            period,
            at_time: parse_at_time(&when)?,
        }
    } else {
        TriggerSpec::Once {
            at: parse_datetime(&when)?,
        }
    };

    let request = CreateScheduleRequest {
        name,
        trigger,
        payload: parse_payload(payload.as_deref())?,
        tags: parse_tags(tags.as_deref()),
        priority,
    };

    let schedule = client.create_schedule(request).await?;
    print_schedule(&schedule);
    Ok(())
}

async fn handle_at(
    time: String,
    name: String,
    payload: Option<String>,
    priority: Priority,
    client: &KairosClient,
) -> Result<()> {
    let at = parse_datetime(&time)?;
    let request = CreateScheduleRequest {
        name,
        trigger: TriggerSpec::Once { at },
        payload: parse_payload(payload.as_deref())?,
        tags: vec![],
        priority,
    };

    let schedule = client.create_schedule(request).await?;
    print_schedule(&schedule);
    Ok(())
}

async fn handle_in(
    duration: String,
    name: String,
    payload: Option<String>,
    priority: Priority,
    client: &KairosClient,
) -> Result<()> {
    let duration_seconds = parse_duration(&duration)?;
    let request = CreateScheduleRequest {
        name,
        trigger: TriggerSpec::In { duration_seconds },
        payload: parse_payload(payload.as_deref())?,
        tags: vec![],
        priority,
    };

    let schedule = client.create_schedule(request).await?;
    print_schedule(&schedule);
    Ok(())
}

async fn handle_every(
    period: Period,
    name: String,
    at: Option<String>,
    payload: Option<String>,
    priority: Priority,
    client: &KairosClient,
) -> Result<()> {
    let request = CreateScheduleRequest {
        name,
        trigger: TriggerSpec::Every {
            period,
            at_time: at.clone(),
        },
        payload: parse_payload(payload.as_deref())?,
        tags: vec![],
        priority,
    };

    let schedule = client.create_schedule(request).await?;
    print_schedule(&schedule);
    Ok(())
}

async fn handle_list(
    status: Option<String>,
    tag: Option<String>,
    client: &KairosClient,
) -> Result<()> {
    let status = parse_status(status.as_deref())?;
    let schedules = client.list_schedules(status, tag.as_deref()).await?;

    if schedules.is_empty() {
        println!("No schedules found.");
    } else {
        for schedule in &schedules {
            print_schedule(schedule);
        }
    }
    Ok(())
}

async fn handle_next(client: &KairosClient) -> Result<()> {
    match client.get_next_schedule().await? {
        Some(schedule) => print_schedule(&schedule),
        None => println!("No upcoming schedules."),
    }
    Ok(())
}

async fn handle_cancel(id: String, client: &KairosClient) -> Result<()> {
    match client.delete_schedule(&id).await? {
        true => println!("Cancelled schedule {}", id),
        false => return Err(anyhow!("Schedule not found: {}", id)),
    }
    Ok(())
}

async fn handle_status(client: &KairosClient) -> Result<()> {
    let status = client.get_status().await?;
    println!("Healthy: {}", status.healthy);
    println!("Active schedules: {}", status.active_schedules);
    println!("Pending triggered: {}", status.pending_triggered);
    if let Some(next) = status.next_fire {
        let fmt = format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
        let next_str = next.format(&fmt).unwrap();
        println!("Next fire: {}", next_str);
    } else {
        println!("No upcoming schedules.");
    }
    Ok(())
}

// === Helper Functions ===

fn print_schedule(schedule: &Schedule) {
    let fmt = format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
    let next_str = schedule
        .next_fire
        .map(|t| t.format(&fmt).unwrap())
        .unwrap_or_default();

    println!("ID:       {}", schedule.id);
    println!("Name:     {}", schedule.name);
    println!("Trigger:  {:?}", schedule.trigger);
    println!("Status:   {:?}", schedule.status);
    println!("Priority: {:?}", schedule.priority);
    println!("Next:     {}", next_str);
    println!("Tags:     {:?}", schedule.tags);
    println!(
        "Payload:  {}",
        serde_json::to_string_pretty(&schedule.payload).unwrap_or_default()
    );
    println!();
}

fn parse_datetime(s: &str) -> Result<OffsetDateTime> {
    // Try RFC3339 first
    if let Ok(dt) = OffsetDateTime::parse(s, &format_description::well_known::Rfc3339) {
        return Ok(dt);
    }

    // Try relative time (e.g., +1h, +30m, +1d) - strictly lowercase
    if let Some(rest) = s.strip_prefix('+') {
        let seconds = parse_relative_time(rest)?;
        return Ok(OffsetDateTime::now_utc() + time::Duration::seconds(seconds));
    }

    Err(anyhow!(
        "Invalid datetime format: {}. Use RFC3339 (e.g., 2026-03-15T14:30:00Z) or relative time (e.g., +1h, +30m)",
        s
    ))
}

fn parse_relative_time(s: &str) -> Result<i64> {
    let mut num = 0u64;
    let mut i = 0;
    let chars = s.as_bytes();

    // Parse number
    while i < chars.len() && chars[i].is_ascii_digit() {
        num = num * 10 + (chars[i] - b'0') as u64;
        i += 1;
    }

    if i == 0 {
        return Err(anyhow!("Invalid relative time: {}", s));
    }

    if i >= chars.len() {
        return Err(anyhow!("Missing unit in relative time: {}", s));
    }

    let unit = std::str::from_utf8(&chars[i..]).unwrap();
    let seconds = match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => num,
        "m" | "min" | "mins" | "minute" | "minutes" => num * 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => num * 3600,
        "d" | "day" | "days" => num * 86400,
        "w" | "wk" | "wks" | "week" | "weeks" => num * 604800,
        _ => return Err(anyhow!("Unknown unit: {}. Use s, m, h, d, w", unit)),
    };

    Ok(seconds as i64)
}

fn parse_duration(s: &str) -> Result<u64> {
    parse_relative_time(s).map(|v| v as u64)
}

fn parse_at_time(s: &str) -> Result<Option<String>> {
    // Simple validation - expect format like "09:00"
    if s.contains(':') && s.len() == 5 {
        Ok(Some(s.to_string()))
    } else if s.is_empty() {
        Ok(None)
    } else {
        Err(anyhow!("Invalid time format: {}. Use HH:MM format (e.g., 09:00)", s))
    }
}

fn parse_payload(s: Option<&str>) -> Result<serde_json::Value> {
    match s {
        Some(json) => {
            if json.is_empty() {
                Ok(serde_json::Value::Null)
            } else {
                serde_json::from_str(json)
                    .map_err(|e| anyhow!("Invalid JSON payload: {}", e))
            }
        }
        None => Ok(serde_json::Value::Null),
    }
}

fn parse_tags(s: Option<&str>) -> Vec<String> {
    match s {
        Some(tags) => tags
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect(),
        None => vec![],
    }
}

fn parse_status(s: Option<&str>) -> Result<Option<ScheduleStatus>> {
    match s {
        Some(status) => {
            let parsed = match status {
                "active" => ScheduleStatus::Active,
                "paused" => ScheduleStatus::Paused,
                "completed" => ScheduleStatus::Completed,
                "triggered" => ScheduleStatus::Triggered,
                _ => return Err(anyhow!("Invalid status: {}. Use active, paused, completed, or triggered", status)),
            };
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    // === parse_relative_time tests ===

    #[test]
    fn test_parse_relative_time_seconds() {
        assert_eq!(parse_relative_time("30s").unwrap(), 30);
        assert_eq!(parse_relative_time("1sec").unwrap(), 1);
        assert_eq!(parse_relative_time("45seconds").unwrap(), 45);
    }

    #[test]
    fn test_parse_relative_time_minutes() {
        assert_eq!(parse_relative_time("5m").unwrap(), 300);
        assert_eq!(parse_relative_time("2min").unwrap(), 120);
        assert_eq!(parse_relative_time("90minutes").unwrap(), 5400);
    }

    #[test]
    fn test_parse_relative_time_hours() {
        assert_eq!(parse_relative_time("1h").unwrap(), 3600);
        assert_eq!(parse_relative_time("24hr").unwrap(), 86400);
        assert_eq!(parse_relative_time("2hours").unwrap(), 7200);
    }

    #[test]
    fn test_parse_relative_time_days() {
        assert_eq!(parse_relative_time("1d").unwrap(), 86400);
        assert_eq!(parse_relative_time("7day").unwrap(), 604800);
        assert_eq!(parse_relative_time("3days").unwrap(), 259200);
    }

    #[test]
    fn test_parse_relative_time_weeks() {
        assert_eq!(parse_relative_time("1w").unwrap(), 604800);
        assert_eq!(parse_relative_time("2wk").unwrap(), 1209600);
        assert_eq!(parse_relative_time("4weeks").unwrap(), 2419200);
    }

    #[test]
    fn test_parse_relative_time_case_sensitive() {
        // Strict case-sensitive: only lowercase accepted
        assert!(parse_relative_time("30S").is_err());
        assert!(parse_relative_time("5M").is_err());
        assert!(parse_relative_time("1H").is_err());
        assert!(parse_relative_time("1D").is_err());
        assert!(parse_relative_time("1W").is_err());
    }

    #[test]
    fn test_parse_relative_time_invalid() {
        assert!(parse_relative_time("").is_err());
        assert!(parse_relative_time("abc").is_err());
        assert!(parse_relative_time("30").is_err());  // Missing unit
        assert!(parse_relative_time("s").is_err());   // Missing number
    }

    // === parse_datetime tests ===

    #[test]
    fn test_parse_datetime_rfc3339() {
        let result = parse_datetime("2026-03-15T14:30:00Z");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_datetime_relative() {
        let result = parse_datetime("+1h");
        assert!(result.is_ok());
        let dt = result.unwrap();
        let now = OffsetDateTime::now_utc();
        let diff = (dt - now).whole_seconds();
        assert!(diff >= 3599 && diff <= 3601);  // ~1 hour
    }

    #[test]
    fn test_parse_datetime_case_sensitive() {
        // Strict case-sensitive: +1H should fail
        assert!(parse_datetime("+1H").is_err());
        assert!(parse_datetime("+30M").is_err());
    }

    #[test]
    fn test_parse_datetime_invalid() {
        assert!(parse_datetime("invalid").is_err());
        assert!(parse_datetime("2026-13-01T00:00:00Z").is_err());  // Invalid month
    }

    // === parse_payload tests ===

    #[test]
    fn test_parse_payload_null() {
        assert!(matches!(parse_payload(None), Ok(serde_json::Value::Null)));
        assert!(matches!(parse_payload(Some("")), Ok(serde_json::Value::Null)));
    }

    #[test]
    fn test_parse_payload_valid_json() {
        let result = parse_payload(Some(r#"{"key": "value"}"#));
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_payload_invalid_json() {
        assert!(parse_payload(Some("not json")).is_err());
        assert!(parse_payload(Some("{invalid}")).is_err());
    }

    // === parse_tags tests ===

    #[test]
    fn test_parse_tags_empty() {
        assert!(parse_tags(None).is_empty());
        assert!(parse_tags(Some("")).is_empty());
    }

    #[test]
    fn test_parse_tags_single() {
        let tags = parse_tags(Some("urgent"));
        assert_eq!(tags, vec!["urgent"]);
    }

    #[test]
    fn test_parse_tags_multiple() {
        let tags = parse_tags(Some("urgent,backup,critical"));
        assert_eq!(tags, vec!["urgent", "backup", "critical"]);
    }

    #[test]
    fn test_parse_tags_with_spaces() {
        let tags = parse_tags(Some("  tag1  ,  tag2  ,  tag3  "));
        assert_eq!(tags, vec!["tag1", "tag2", "tag3"]);
    }

    // === parse_status tests (kairos-cli version) ===

    #[test]
    fn test_cli_parse_status_valid() {
        assert!(matches!(parse_status(Some("active")), Ok(Some(ScheduleStatus::Active))));
        assert!(matches!(parse_status(Some("paused")), Ok(Some(ScheduleStatus::Paused))));
        assert!(matches!(parse_status(Some("completed")), Ok(Some(ScheduleStatus::Completed))));
        assert!(matches!(parse_status(Some("triggered")), Ok(Some(ScheduleStatus::Triggered))));
    }

    #[test]
    fn test_cli_parse_status_none() {
        assert!(matches!(parse_status(None), Ok(None)));
    }

    #[test]
    fn test_cli_parse_status_invalid() {
        assert!(parse_status(Some("ACTIVE")).is_err());  // Case-sensitive
        assert!(parse_status(Some("invalid")).is_err());
    }
}
