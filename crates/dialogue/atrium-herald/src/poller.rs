//! Poller for atrium messages.

use atrium_client::{AuthenticatedClient, Message};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

pub struct Poller {
    atrium_client: AuthenticatedClient,
    http_client: Client,
    herald_id: String,
    agora_url: String,
    poll_interval: Duration,
    agora_heartbeat_interval: Duration,
    atrium_heartbeat_interval: Duration,
}

impl Poller {
    pub fn new(
        atrium_client: AuthenticatedClient,
        herald_id: &str,
        agora_url: &str,
        poll_interval: Duration,
        agora_heartbeat_interval: Duration,
        atrium_heartbeat_interval: Duration,
        http_client: Client,
    ) -> Self {
        Self {
            atrium_client,
            http_client,
            herald_id: herald_id.to_string(),
            agora_url: agora_url.to_string(),
            poll_interval,
            agora_heartbeat_interval,
            atrium_heartbeat_interval,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Register with Agora
        self.register().await?;

        let mut poll_tick = interval(self.poll_interval);
        let mut agora_heartbeat_tick = interval(self.agora_heartbeat_interval);
        let mut atrium_heartbeat_tick = interval(self.atrium_heartbeat_interval);

        info!("Starting atrium-herald loop");

        loop {
            tokio::select! {
                _ = poll_tick.tick() => {
                    if let Err(e) = self.poll_and_push().await {
                        warn!("Poll error: {}", e);
                    }
                }
                _ = agora_heartbeat_tick.tick() => {
                    if let Err(e) = self.send_agora_heartbeat().await {
                        warn!("Agora heartbeat error: {}", e);
                    }
                }
                _ = atrium_heartbeat_tick.tick() => {
                    if let Err(e) = self.send_atrium_heartbeat().await {
                        warn!("Atrium heartbeat error: {}", e);
                    }
                }
            }
        }
    }

    async fn register(&self) -> anyhow::Result<()> {
        info!("Registering herald: {}", self.herald_id);

        let url = format!("{}/heralds", self.agora_url);
        let body = json!({
            "id": self.herald_id,
            "description": "Atrium Herald - produces chat.message events"
        });

        let response = self.http_client.post(&url).json(&body).send().await?;

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

    async fn send_agora_heartbeat(&self) -> anyhow::Result<()> {
        debug!("Sending heartbeat to Agora");

        let url = format!("{}/heralds/{}/heartbeat", self.agora_url, self.herald_id);
        let response = self.http_client.put(&url).send().await?;

        if response.status().is_success() {
            debug!("Agora heartbeat sent successfully");
            Ok(())
        } else {
            let status = response.status();
            warn!("Agora heartbeat failed: HTTP {}", status);
            Err(anyhow::anyhow!("Agora heartbeat failed: HTTP {}", status))
        }
    }

    async fn send_atrium_heartbeat(&self) -> anyhow::Result<()> {
        debug!("Sending heartbeat to Atrium");
        self.atrium_client
            .send_heartbeat()
            .await
            .map_err(|e| anyhow::anyhow!("Atrium heartbeat failed: {}", e))?;
        debug!("Atrium heartbeat sent successfully");
        Ok(())
    }

    async fn poll_and_push(&self) -> anyhow::Result<()> {
        // Get unread messages from atrium
        let unread = self.atrium_client.get_unread_messages(Some(100)).await?;

        if unread.messages.is_empty() {
            debug!("No new messages");
            return Ok(());
        }

        info!("Found {} new messages", unread.messages.len());

        // Push each message as an event
        for msg in unread.messages {
            if let Err(e) = self.push_message_event(msg).await {
                error!("Failed to push message event: {}", e);
            }
        }

        Ok(())
    }

    async fn push_message_event(&self, msg: Message) -> anyhow::Result<()> {
        let url = format!("{}/events", self.agora_url);
        let timestamp = msg.created_at;

        let body = json!({
            "event_type": "chat.message",
            "herald_id": self.herald_id,
            "priority": "normal",
            "timestamp": timestamp,
            "payload": {
                "id": msg.id,
                "content": msg.content,
                "sender": msg.sender,
                "created_at": timestamp,
            }
        });

        debug!(
            "Pushing chat.message event: id={}, sender={}",
            msg.id, msg.sender
        );

        let response = self.http_client.post(&url).json(&body).send().await?;

        if response.status().is_success() {
            debug!("Event pushed successfully for message {}", msg.id);
            Ok(())
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Failed to push event: HTTP {} - {}", status, text);
            Err(anyhow::anyhow!("Failed to push event: HTTP {}", status))
        }
    }
}
