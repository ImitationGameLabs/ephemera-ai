//! HTTP client for Kairos time management service.

use reqwest::{Client, Error as ReqwestError, Response};
use serde::de::DeserializeOwned;
use std::fmt;
use tracing::{debug, instrument};

use kairos_common::schedule::{
    AckTriggeredRequest, CreateScheduleRequest, Schedule, ScheduleStatus,
    SchedulesListResponse, StatusResponse, TriggeredSchedule, UpdateScheduleRequest,
};

/// Client error types.
#[derive(Debug)]
pub enum KairosClientError {
    NetworkError(ReqwestError),
    ApiError(String),
    JsonError(serde_json::Error),
}

impl fmt::Display for KairosClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KairosClientError::NetworkError(e) => write!(f, "Network error: {}", e),
            KairosClientError::ApiError(msg) => write!(f, "API error: {}", msg),
            KairosClientError::JsonError(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for KairosClientError {}

impl From<ReqwestError> for KairosClientError {
    fn from(error: ReqwestError) -> Self {
        KairosClientError::NetworkError(error)
    }
}

impl From<serde_json::Error> for KairosClientError {
    fn from(error: serde_json::Error) -> Self {
        KairosClientError::JsonError(error)
    }
}

/// HTTP client for Kairos time management service.
#[derive(Clone)]
pub struct KairosClient {
    client: Client,
    base_url: String,
}

impl KairosClient {
    /// Creates a new Kairos client with the given base URL and HTTP client.
    pub fn new(base_url: &str, client: Client) -> Self {
        Self {
            client,
            base_url: base_url.to_string(),
        }
    }

    /// Handles HTTP response and converts to expected type.
    async fn handle_response<T: DeserializeOwned>(
        response: Response,
    ) -> Result<T, KairosClientError> {
        let status = response.status();
        let url = response.url().clone();

        debug!("Received response from {}: {}", url, status);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            return Err(KairosClientError::ApiError(format!(
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
    pub async fn health_check(&self) -> Result<String, KairosClientError> {
        let url = format!("{}/health", self.base_url);
        debug!("Making health check request to: {}", url);

        let response = self.client.get(&url).send().await?;
        let result: String = Self::handle_response(response).await?;
        Ok(result)
    }

    /// Gets the service status.
    #[instrument(skip(self))]
    pub async fn get_status(&self) -> Result<StatusResponse, KairosClientError> {
        let url = format!("{}/status", self.base_url);
        debug!("Getting status from: {}", url);

        let response = self.client.get(&url).send().await?;
        let status: StatusResponse = Self::handle_response(response).await?;
        Ok(status)
    }

    // === Schedule operations ===

    /// Creates a new schedule.
    #[instrument(skip(self))]
    pub async fn create_schedule(
        &self,
        request: CreateScheduleRequest,
    ) -> Result<Schedule, KairosClientError> {
        let url = format!("{}/schedules", self.base_url);
        debug!("Creating schedule at: {}", url);

        let response = self.client.post(&url).json(&request).send().await?;
        let schedule: Schedule = Self::handle_response(response).await?;
        Ok(schedule)
    }

    /// Lists schedules with optional filtering.
    #[instrument(skip(self))]
    pub async fn list_schedules(
        &self,
        status: Option<ScheduleStatus>,
        tag: Option<&str>,
    ) -> Result<Vec<Schedule>, KairosClientError> {
        let url = format!("{}/schedules", self.base_url);
        debug!("Listing schedules from: {}", url);

        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(s) = status {
            query.push(("status", s.to_string()));
        }
        if let Some(t) = tag {
            query.push(("tag", t.to_string()));
        }

        let request = self.client.get(&url).query(&query);
        let response: SchedulesListResponse = Self::handle_response(request.send().await?).await?;

        Ok(response.schedules)
    }

    /// Gets a specific schedule by ID.
    #[instrument(skip(self))]
    pub async fn get_schedule(&self, id: &str) -> Result<Option<Schedule>, KairosClientError> {
        let url = format!("{}/schedules/{}", self.base_url, id);
        debug!("Getting schedule from: {}", url);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        let schedule: Schedule = Self::handle_response(response).await?;
        Ok(Some(schedule))
    }

    /// Gets the next schedule to fire.
    #[instrument(skip(self))]
    pub async fn get_next_schedule(&self) -> Result<Option<Schedule>, KairosClientError> {
        let url = format!("{}/schedules/next", self.base_url);
        debug!("Getting next schedule from: {}", url);

        let response = self.client.get(&url).send().await?;
        let result: Option<Schedule> = Self::handle_response(response).await?;
        Ok(result)
    }

    /// Deletes a schedule.
    #[instrument(skip(self))]
    pub async fn delete_schedule(&self, id: &str) -> Result<bool, KairosClientError> {
        let url = format!("{}/schedules/{}", self.base_url, id);
        debug!("Deleting schedule at: {}", url);

        let response = self.client.delete(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(false);
        }

        Self::handle_response::<serde_json::Value>(response).await?;
        Ok(true)
    }

    /// Updates a schedule.
    #[instrument(skip(self))]
    pub async fn update_schedule(
        &self,
        id: &str,
        request: UpdateScheduleRequest,
    ) -> Result<Schedule, KairosClientError> {
        let url = format!("{}/schedules/{}", self.base_url, id);
        debug!("Updating schedule at: {}", url);

        let response = self.client.patch(&url).json(&request).send().await?;
        let schedule: Schedule = Self::handle_response(response).await?;
        Ok(schedule)
    }

    // === Triggered schedule operations (for kairos-herald) ===

    /// Gets triggered schedules (ready to be pushed to Agora).
    #[instrument(skip(self))]
    pub async fn get_triggered(&self) -> Result<Vec<TriggeredSchedule>, KairosClientError> {
        let url = format!("{}/schedules/triggered", self.base_url);
        debug!("Getting triggered schedules from: {}", url);

        let response = self.client.get(&url).send().await?;
        let triggered: Vec<TriggeredSchedule> = Self::handle_response(response).await?;
        Ok(triggered)
    }

    /// Acknowledges triggered schedules.
    #[instrument(skip(self))]
    pub async fn ack_triggered(&self, ids: Vec<String>) -> Result<usize, KairosClientError> {
        let url = format!("{}/schedules/triggered/ack", self.base_url);
        debug!("Acknowledging {} triggered schedules at: {}", ids.len(), url);

        let request = AckTriggeredRequest { ids };
        let response = self.client.post(&url).json(&request).send().await?;
        let result: serde_json::Value = Self::handle_response(response).await?;

        let acknowledged = result["acknowledged"].as_u64().unwrap_or(0) as usize;
        Ok(acknowledged)
    }

    /// Gets the base URL this client is configured to use.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let http_client = Client::new();
        let client = KairosClient::new("http://localhost:8081", http_client);
        assert_eq!(client.base_url(), "http://localhost:8081");
    }
}
