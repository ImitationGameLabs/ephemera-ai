use reqwest::{Client, Response, Error as ReqwestError};
use serde::de::DeserializeOwned;
use std::fmt;
use tracing::{debug, instrument};

use loom::memory::models::*;

/// Client error types
#[derive(Debug)]
pub enum LoomClientError {
    NetworkError(ReqwestError),
    ApiError(String),
    JsonError(serde_json::Error),
}

impl fmt::Display for LoomClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LoomClientError::NetworkError(e) => write!(f, "Network error: {}", e),
            LoomClientError::ApiError(msg) => write!(f, "API error: {}", msg),
            LoomClientError::JsonError(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for LoomClientError {}

impl From<ReqwestError> for LoomClientError {
    fn from(error: ReqwestError) -> Self {
        LoomClientError::NetworkError(error)
    }
}

impl From<serde_json::Error> for LoomClientError {
    fn from(error: serde_json::Error) -> Self {
        LoomClientError::JsonError(error)
    }
}

/// HTTP client for Loom memory service
#[derive(Clone)]
pub struct LoomClient {
    client: Client,
    base_url: String,
}

impl LoomClient {
    /// Create a new Loom client with the given base URL
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Create a new Loom client with custom HTTP client configuration
    pub fn with_client(base_url: impl Into<String>, client: Client) -> Self {
        Self {
            client,
            base_url: base_url.into(),
        }
    }

    /// Handle HTTP response and convert to expected type
    async fn handle_response<T: DeserializeOwned>(
        response: Response,
    ) -> Result<T, LoomClientError> {
        let status = response.status();
        let url = response.url().clone();

        debug!("Received response from {}: {}", url, status);

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            return Err(LoomClientError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let text = response.text().await?;
        let result: T = serde_json::from_str(&text)?;
        Ok(result)
    }

    /// Health check - verify the service is running
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<serde_json::Value, LoomClientError> {
        let url = format!("{}/health", self.base_url);
        debug!("Making health check request to: {}", url);

        let response = self.client.get(&url).send().await?;
        Self::handle_response(response).await
    }

    /// Create a new memory fragment
    #[instrument(skip(self))]
    pub async fn create_memory(
        &self,
        request: CreateMemoryRequest,
    ) -> Result<MemoryResponse, LoomClientError> {
        let url = format!("{}/api/v1/memory", self.base_url);
        debug!("Creating memory fragment at: {}", url);

        let response = self.client.post(&url).json(&request).send().await?;
        let api_response: ApiResponse<MemoryResponse> = Self::handle_response(response).await?;

        api_response
            .data
            .ok_or_else(|| LoomClientError::ApiError("No data returned from API".to_string()))
    }

    /// Search memory fragments
    #[instrument(skip(self))]
    pub async fn search_memory(
        &self,
        request: SearchMemoryRequest,
    ) -> Result<SearchMemoryResponse, LoomClientError> {
        let url = format!("{}/api/v1/memory", self.base_url);
        debug!("Searching memory fragments at: {}", url);

        let response = self.client.get(&url).query(&request).send().await?;
        let api_response: ApiResponse<SearchMemoryResponse> = Self::handle_response(response).await?;

        api_response
            .data
            .ok_or_else(|| LoomClientError::ApiError("No data returned from API".to_string()))
    }

    /// Get a specific memory fragment by ID
    #[instrument(skip(self))]
    pub async fn get_memory(&self, id: i64) -> Result<MemoryResponse, LoomClientError> {
        let url = format!("{}/api/v1/memory/{}", self.base_url, id);
        debug!("Getting memory fragment from: {}", url);

        let response = self.client.get(&url).send().await?;
        let api_response: ApiResponse<MemoryResponse> = Self::handle_response(response).await?;

        api_response
            .data
            .ok_or_else(|| LoomClientError::ApiError("No data returned from API".to_string()))
    }

    /// Delete a memory fragment by ID
    #[instrument(skip(self))]
    pub async fn delete_memory(&self, id: i64) -> Result<(), LoomClientError> {
        let url = format!("{}/api/v1/memory/{}", self.base_url, id);
        debug!("Deleting memory fragment at: {}", url);

        let response = self.client.delete(&url).send().await?;
        let _: ApiResponse<serde_json::Value> = Self::handle_response(response).await?;

        Ok(())
    }

    /// Get the base URL this client is configured to use
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = LoomClient::new("http://localhost:8080");
        assert_eq!(client.base_url(), "http://localhost:8080");
    }

    #[test]
    fn test_client_with_custom_http_client() {
        let http_client = Client::new();
        let client = LoomClient::with_client("http://localhost:8080", http_client);
        assert_eq!(client.base_url(), "http://localhost:8080");
    }
}