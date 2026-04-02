//! HTTP client for Agora event hub.

use async_trait::async_trait;
use reqwest::{Client, Error as ReqwestError, Response};
use serde::de::DeserializeOwned;
use std::fmt;
use tracing::{debug, instrument};

use agora_common::event::{Event, EventsListResponse};
use agora_common::herald::{HeraldInfo, HeraldsListResponse};

use crate::AgoraClientTrait;

/// Client error types.
#[derive(Debug)]
pub enum AgoraClientError {
    NetworkError(ReqwestError),
    ApiError(String),
    JsonError(serde_json::Error),
}

impl fmt::Display for AgoraClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AgoraClientError::NetworkError(e) => write!(f, "Network error: {}", e),
            AgoraClientError::ApiError(msg) => write!(f, "API error: {}", msg),
            AgoraClientError::JsonError(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for AgoraClientError {}

impl From<ReqwestError> for AgoraClientError {
    fn from(error: ReqwestError) -> Self {
        AgoraClientError::NetworkError(error)
    }
}

impl From<serde_json::Error> for AgoraClientError {
    fn from(error: serde_json::Error) -> Self {
        AgoraClientError::JsonError(error)
    }
}

/// HTTP client for Agora event hub.
#[derive(Clone)]
pub struct AgoraClient {
    client: Client,
    base_url: String,
}

impl AgoraClient {
    /// Creates a new Agora client with the given base URL and HTTP client.
    pub fn new(base_url: &str, client: Client) -> Self {
        Self { client, base_url: base_url.to_string() }
    }

    /// Handles HTTP response and converts to expected type.
    async fn handle_response<T: DeserializeOwned>(
        response: Response,
    ) -> Result<T, AgoraClientError> {
        let status = response.status();
        let url = response.url().clone();

        debug!("Received response from {}: {}", url, status);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            return Err(AgoraClientError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let text = response.text().await?;
        let result: T = serde_json::from_str(&text)?;
        Ok(result)
    }

    /// Health check - verifies the service is running.
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<String, AgoraClientError> {
        let url = format!("{}/health", self.base_url);
        debug!("Making health check request to: {}", url);

        let response = self.client.get(&url).send().await?;
        let result: String = Self::handle_response(response).await?;
        Ok(result)
    }

    // === Event operations ===

    /// Fetches events for delivery (POST /events/fetch).
    /// This changes state: Pending → Delivered.
    #[instrument(skip(self))]
    pub async fn fetch_events(&self, limit: Option<u32>) -> Result<Vec<Event>, AgoraClientError> {
        let url = format!("{}/events/fetch", self.base_url);
        debug!("Fetching events from: {}", url);

        let body = serde_json::json!({ "limit": limit.unwrap_or(10) });
        let response = self.client.post(&url).json(&body).send().await?;
        let result: EventsListResponse = Self::handle_response(response).await?;

        Ok(result.events)
    }

    /// Acknowledges a single event.
    #[instrument(skip(self))]
    pub async fn ack_event(&self, event_id: u64) -> Result<Event, AgoraClientError> {
        let url = format!("{}/events/{}", self.base_url, event_id);
        debug!("Acknowledging event at: {}", url);

        let body = serde_json::json!({ "status": "acked" });
        let response = self.client.patch(&url).json(&body).send().await?;
        let event: Event = Self::handle_response(response).await?;

        Ok(event)
    }

    /// Batch acknowledges multiple events.
    #[instrument(skip(self))]
    pub async fn ack_events(&self, event_ids: Vec<u64>) -> Result<usize, AgoraClientError> {
        let url = format!("{}/events", self.base_url);
        debug!("Batch acknowledging {} events at: {}", event_ids.len(), url);

        let body = serde_json::json!({
            "event_ids": event_ids,
            "status": "acked"
        });
        let response = self.client.patch(&url).json(&body).send().await?;
        let result: serde_json::Value = Self::handle_response(response).await?;

        let updated = result["updated"].as_u64().unwrap_or(0) as usize;
        Ok(updated)
    }

    // === Herald operations ===

    /// Lists all heralds.
    #[instrument(skip(self))]
    pub async fn list_heralds(&self) -> Result<Vec<HeraldInfo>, AgoraClientError> {
        let url = format!("{}/heralds", self.base_url);
        debug!("Listing heralds from: {}", url);

        let response = self.client.get(&url).send().await?;
        let result: HeraldsListResponse = Self::handle_response(response).await?;

        Ok(result.heralds)
    }

    /// Gets a specific herald by ID.
    #[instrument(skip(self))]
    pub async fn get_herald(&self, id: &str) -> Result<HeraldInfo, AgoraClientError> {
        let url = format!("{}/heralds/{}", self.base_url, id);
        debug!("Getting herald from: {}", url);

        let response = self.client.get(&url).send().await?;
        let herald: HeraldInfo = Self::handle_response(response).await?;

        Ok(herald)
    }

    /// Gets the base URL this client is configured to use.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[async_trait]
impl AgoraClientTrait for AgoraClient {
    async fn health_check(&self) -> Result<String, AgoraClientError> {
        AgoraClient::health_check(self).await
    }

    async fn fetch_events(&self, limit: Option<u32>) -> Result<Vec<Event>, AgoraClientError> {
        AgoraClient::fetch_events(self, limit).await
    }

    async fn ack_event(&self, event_id: u64) -> Result<Event, AgoraClientError> {
        AgoraClient::ack_event(self, event_id).await
    }

    async fn ack_events(&self, event_ids: Vec<u64>) -> Result<usize, AgoraClientError> {
        AgoraClient::ack_events(self, event_ids).await
    }

    async fn list_heralds(&self) -> Result<Vec<HeraldInfo>, AgoraClientError> {
        AgoraClient::list_heralds(self).await
    }

    async fn get_herald(&self, id: &str) -> Result<HeraldInfo, AgoraClientError> {
        AgoraClient::get_herald(self, id).await
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let http_client = Client::new();
        let client = AgoraClient::new("http://localhost:8080", http_client);
        assert_eq!(client.base_url(), "http://localhost:8080");
    }
}
