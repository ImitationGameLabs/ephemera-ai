//! Timer scheduler for producing timer events.

use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

/// Timer scheduler that produces timer events.
pub struct TimerScheduler {
    client: Client,
    herald_id: String,
    heartbeat_interval: Duration,
    tick_interval: Duration,
    agora_url: String,
}

impl TimerScheduler {
    /// Creates a new timer scheduler.
    pub fn new(
        agora_url: &str,
        herald_id: &str,
        heartbeat_interval: Duration,
        tick_interval: Duration,
    ) -> Self {
        Self {
            client: Client::new(),
            herald_id: herald_id.to_string(),
            heartbeat_interval,
            tick_interval,
            agora_url: agora_url.to_string(),
        }
    }

    /// Registers this herald with Agora.
    async fn register(&self) -> anyhow::Result<()> {
        info!("Registering herald: {}", self.herald_id);

        let url = format!("{}/heralds", self.agora_url);
        let body = json!({
            "id": self.herald_id,
            "description": "Timer Herald - produces timer.tick events"
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            info!("Herald registered successfully");
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Failed to register herald: HTTP {} - {}", status, text);
            Err(anyhow::anyhow!(
                "Failed to register herald: HTTP {}",
                status
            ))
        }
    }

    /// Sends a heartbeat to Agora.
    async fn send_heartbeat(&self) -> anyhow::Result<()> {
        debug!("Sending heartbeat");

        let url = format!(
            "{}/heralds/{}/heartbeat",
            self.agora_url,
            self.herald_id
        );
        let response = self.client
            .put(&url)
            .send()
            .await?;

        if response.status().is_success() {
            debug!("Heartbeat sent successfully");
            Ok(())
        } else {
            let status = response.status();
            warn!("Heartbeat failed: HTTP {}", status);
            Err(anyhow::anyhow!("Heartbeat failed: HTTP {}", status))
        }
    }

    /// Pushes a timer.tick event to Agora.
    async fn push_tick_event(&self) -> anyhow::Result<()> {
        let now = OffsetDateTime::now_utc();
        info!("Pushing timer.tick event at {}", now);

        let url = format!("{}/events", self.agora_url);
        let body = json!({
            "event_type": "timer.tick",
            "herald_id": self.herald_id,
            "priority": "normal",
            "timestamp": now,
            "payload": {
                "tick_time": now.unix_timestamp(),
            }
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            info!("Timer.tick event pushed successfully");
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Failed to push event: HTTP {} - {}", status, text);
            Err(anyhow::anyhow!("Failed to push event: HTTP {}", status))
        }
    }

    /// Runs the scheduler loop.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Register first
        self.register().await?;

        // Set up intervals
        let mut heartbeat_tick = interval(self.heartbeat_interval);
        let mut timer_tick = interval(self.tick_interval);

        info!("Starting timer scheduler loop");

        loop {
            tokio::select! {
                _ = heartbeat_tick.tick() => {
                    if let Err(e) = self.send_heartbeat().await {
                        warn!("Heartbeat error: {}", e);
                    }
                }
                _ = timer_tick.tick() => {
                    if let Err(e) = self.push_tick_event().await {
                        error!("Failed to push timer event: {}", e);
                    }
                }
            }
        }
    }
}
