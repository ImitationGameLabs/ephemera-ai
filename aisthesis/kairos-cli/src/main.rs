use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
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
        /// When to trigger (RFC3339 timestamp or relative like +1h, +30m)
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
        /// Duration (e.g., 30s, 5m, 2h, 1d)
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
        /// Filter by status (active, paused, completed, triggered)
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
            print_schedule(&schedule);
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

    // Try relative time (e.g., +1h, +30m, +1d)
    let lower = s.to_lowercase();
    if lower.starts_with('+') {
        let rest = &lower[1..];
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

    let unit = std::str::from_utf8(&chars[i..]).unwrap().to_lowercase();
    let seconds = match unit.as_str() {
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
            let status = match status.to_lowercase().as_str() {
                "active" => ScheduleStatus::Active,
                "paused" => ScheduleStatus::Paused,
                "completed" => ScheduleStatus::Completed,
                "triggered" => ScheduleStatus::Triggered,
                _ => return Err(anyhow!("Invalid status: {}. Use active, paused, completed, or triggered", status)),
            };
            Ok(Some(status))
        }
        None => Ok(None),
    }
}
