//! Kairos Herald - Bridge service between Kairos and Agora.
//!
//! This stateless service:
//! 1. Registers with Agora as a herald
//! 2. Polls Kairos for triggered schedules
//! 3. Pushes events to Agora
//! 4. Acknowledges the triggered schedules

use anyhow::Result;
use clap::Parser;
use kairos_client::KairosClient;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use tracing_subscriber::prelude::*;

const HERALD_ID: &str = "kairos-herald";

/// Kairos Herald configuration.
#[derive(Parser)]
#[command(name = "kairos-herald")]
struct Args {
    /// Kairos server URL.
    #[arg(long, default_value = "http://localhost:8081")]
    kairos_url: String,

    /// Agora server URL.
    #[arg(long, default_value = "http://localhost:8080")]
    agora_url: String,

    /// Poll interval for triggered schedules (milliseconds).
    #[arg(long, default_value = "1000")]
    poll_interval_ms: u64,

    /// Heartbeat interval in seconds.
    #[arg(long, default_value = "30")]
    heartbeat_interval: u64,
}

fn build_http_client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kairos_herald=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();
    info!("Starting Kairos Herald (bridge service)");
    info!("Kairos URL: {}", args.kairos_url);
    info!("Agora URL: {}", args.agora_url);
    info!("Poll interval: {}ms", args.poll_interval_ms);
    info!("Heartbeat interval: {}s", args.heartbeat_interval);

    // Create clients
    let http_client = build_http_client();
    let kairos_client = KairosClient::new(&args.kairos_url, http_client.clone());

    // Register with Agora
    register_herald(&http_client, &args.agora_url).await?;

    // Run main loop
    let poll_interval = Duration::from_millis(args.poll_interval_ms);
    let heartbeat_interval = Duration::from_secs(args.heartbeat_interval);
    let mut poll_ticker = tokio::time::interval(poll_interval);
    let mut heartbeat_ticker = tokio::time::interval(heartbeat_interval);

    loop {
        tokio::select! {
            _ = poll_ticker.tick() => {
                if let Err(e) = process_triggered_schedules(&kairos_client, &http_client, &args.agora_url).await {
                    error!("Error processing triggered schedules: {}", e);
                }
            }
            _ = heartbeat_ticker.tick() => {
                if let Err(e) = send_heartbeat(&http_client, &args.agora_url).await {
                    warn!("Failed to send heartbeat: {}", e);
                }
            }
        }
    }
}

async fn register_herald(client: &Client, agora_url: &str) -> Result<()> {
    let url = format!("{}/heralds", agora_url);
    let body = json!({
        "id": HERALD_ID,
        "description": "Kairos Herald - pushes triggered schedules to Agora"
    });

    let response = client.post(&url).json(&body).send().await?;

    if response.status().is_success() {
        info!("Registered herald '{}' with Agora", HERALD_ID);
        Ok(())
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        Err(anyhow::anyhow!("Failed to register herald: HTTP {} - {}", status, text))
    }
}

async fn send_heartbeat(client: &Client, agora_url: &str) -> Result<()> {
    let url = format!("{}/heralds/{}/heartbeat", agora_url, HERALD_ID);
    let response = client.post(&url).send().await?;

    if response.status().is_success() {
        debug!("Heartbeat sent successfully");
        Ok(())
    } else {
        let status = response.status();
        Err(anyhow::anyhow!("Heartbeat failed: HTTP {}", status))
    }
}

async fn process_triggered_schedules(
    kairos_client: &KairosClient,
    http_client: &Client,
    agora_url: &str,
) -> Result<()> {
    // Get triggered schedules from Kairos
    let triggered = kairos_client.get_triggered().await?;

    if triggered.is_empty() {
        return Ok(());
    }

    debug!("Processing {} triggered schedules", triggered.len());

    let mut processed_ids = Vec::new();

    for item in triggered {
        let schedule = item.schedule;
        let triggered_at = item.triggered_at;

        info!(
            "Pushing triggered schedule '{}' ({}) to Agora",
            schedule.name, schedule.id
        );

        // Create event for Agora
        let event_body = json!({
            "event_type": "kairos.trigger",
            "herald_id": HERALD_ID,
            "priority": schedule.priority,
            "timestamp": triggered_at,
            "payload": {
                "schedule_id": schedule.id,
                "schedule_name": schedule.name,
                "tags": schedule.tags,
                "user_payload": schedule.payload,
                "triggered_at": triggered_at,
            }
        });

        let events_url = format!("{}/events", agora_url);
        let response = http_client.post(&events_url).json(&event_body).send().await?;

        if response.status().is_success() {
            debug!("Event pushed successfully for schedule {}", schedule.id);
            processed_ids.push(schedule.id);
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!(
                "Failed to push event for schedule {}: HTTP {} - {}",
                schedule.id, status, text
            );
        }
    }

    // Acknowledge processed schedules
    if !processed_ids.is_empty() {
        debug!("Acknowledging {} processed schedules", processed_ids.len());
        let acknowledged = kairos_client.ack_triggered(processed_ids).await?;
        debug!("Acknowledged {} schedules", acknowledged);
    }

    Ok(())
}
