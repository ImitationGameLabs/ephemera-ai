//! Kairos Herald - Bridge service between Kairos and Agora.
//!
//! This stateless service:
//! 1. Registers with Agora as a herald
//! 2. Polls Kairos for triggered schedules
//! 3. Pushes events to Agora
//! 4. Acknowledges the triggered schedules

mod config;

use anyhow::Result;
use clap::Parser;
use kairos_client::KairosClient;
use reqwest::Client;
use serde_json::json;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use tracing_subscriber::prelude::*;

use crate::config::Config;

const HERALD_ID: &str = "kairos-herald";

/// Kairos Herald configuration.
#[derive(Parser)]
#[command(name = "kairos-herald")]
#[command(about = "Kairos herald for Agora event hub")]
struct Args {
    /// Config directory path
    #[arg(long, default_value = ".")]
    config_dir: PathBuf,
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
    let config_path = args.config_dir.join("config.json");
    let config = Config::load(&config_path);

    info!("Starting Kairos Herald (bridge service)");
    info!("Kairos URL: {}", config.kairos_url);
    info!("Agora URL: {}", config.agora_url);
    info!("Poll interval: {}ms", config.poll_interval_ms);
    info!("Heartbeat interval: {}s", config.heartbeat_interval_sec);

    // Create clients
    let http_client = build_http_client();
    let kairos_client = KairosClient::new(&config.kairos_url, http_client.clone());

    // Register with Agora
    let mut attempt = 0u32;
    let mut delay = Duration::from_secs(1);
    loop {
        match register_herald(&http_client, &config.agora_url).await {
            Ok(_) => break,
            Err(e) => {
                attempt += 1;
                warn!(
                    "Agora registration failed (attempt {attempt}): {e}. Retrying in {}s...",
                    delay.as_secs()
                );
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(30));
            }
        }
    }

    // Run main loop
    let poll_interval = Duration::from_millis(config.poll_interval_ms);
    let heartbeat_interval = Duration::from_secs(config.heartbeat_interval_sec);
    let mut poll_ticker = tokio::time::interval(poll_interval);
    let mut heartbeat_ticker = tokio::time::interval(heartbeat_interval);

    loop {
        tokio::select! {
            _ = poll_ticker.tick() => {
                if let Err(e) = process_triggered_schedules(&kairos_client, &http_client, &config.agora_url).await {
                    error!("Error processing triggered schedules: {}", e);
                }
            }
            _ = heartbeat_ticker.tick() => {
                if let Err(e) = send_heartbeat(&http_client, &config.agora_url).await {
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
        Err(anyhow::anyhow!(
            "Failed to register herald: HTTP {} - {}",
            status,
            text
        ))
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
        let response = http_client
            .post(&events_url)
            .json(&event_body)
            .send()
            .await?;

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
