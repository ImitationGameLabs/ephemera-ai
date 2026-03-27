use async_trait::async_trait;
use reqwest::{Client, Error as ReqwestError, Response};
use serde::de::DeserializeOwned;
use std::fmt;
use tracing::{debug, instrument};

use loom::memory::models::*;
use loom::memory::types::MemoryFragment;

use crate::LoomClientTrait;

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
    /// Create a new Loom client with the given base URL and HTTP client
    pub fn new(base_url: &str, client: Client) -> Self {
        Self { client, base_url: base_url.to_string() }
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
            return Err(LoomClientError::ApiError(format!("HTTP {}: {}", status, error_text)));
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
        let url = format!("{}/api/v1/memories", self.base_url);
        debug!("Creating {} memory fragments at: {}", request.fragments.len(), url);

        let response = self.client.post(&url).json(&request).send().await?;
        let api_response: ApiResponse<MemoryResponse> = Self::handle_response(response).await?;

        api_response
            .data
            .ok_or_else(|| LoomClientError::ApiError("No data returned from API".to_string()))
    }

    /// Create a single memory fragment (backward compatibility convenience method)
    #[instrument(skip(self))]
    pub async fn create_single_memory(
        &self,
        fragment: MemoryFragment,
    ) -> Result<MemoryResponse, LoomClientError> {
        let request = CreateMemoryRequest::single(fragment);
        self.create_memory(request).await
    }

    /// Get a specific memory fragment by ID
    #[instrument(skip(self))]
    pub async fn get_memory(&self, id: i64) -> Result<MemoryResponse, LoomClientError> {
        let url = format!("{}/api/v1/memories/{}", self.base_url, id);
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
        let url = format!("{}/api/v1/memories/{}", self.base_url, id);
        debug!("Deleting memory fragment at: {}", url);

        let response = self.client.delete(&url).send().await?;
        let _: ApiResponse<serde_json::Value> = Self::handle_response(response).await?;

        Ok(())
    }

    /// Get recent memory fragments
    #[instrument(skip(self))]
    pub async fn get_recent_memories(
        &self,
        limit: usize,
    ) -> Result<MemoryResponse, LoomClientError> {
        let url = format!("{}/api/v1/memories/views/recent", self.base_url);
        debug!("Getting {} recent memory fragments from: {}", limit, url);

        let response = self.client.get(&url).query(&[("limit", limit)]).send().await?;
        let api_response: ApiResponse<MemoryResponse> = Self::handle_response(response).await?;

        api_response
            .data
            .ok_or_else(|| LoomClientError::ApiError("No data returned from API".to_string()))
    }

    /// Get memory fragments within a time range (timeline view)
    ///
    /// Time format: ISO 8601 (e.g., "2024-01-15T10:30:00Z" or "2024-01-15T10:30:00+08:00")
    #[instrument(skip(self))]
    pub async fn get_timeline_memory(
        &self,
        from: &str,
        to: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<MemoryResponse, LoomClientError> {
        let url = format!("{}/api/v1/memories/views/timeline", self.base_url);
        debug!("Getting memory fragments in range {} to {} from: {}", from, to, url);

        let mut query: Vec<(&str, String)> =
            vec![("from", from.to_string()), ("to", to.to_string())];

        if let Some(limit) = limit {
            query.push(("limit", limit.to_string()));
        }

        if let Some(offset) = offset {
            query.push(("offset", offset.to_string()));
        }

        let response = self.client.get(&url).query(&query).send().await?;
        let api_response: ApiResponse<MemoryResponse> = Self::handle_response(response).await?;

        api_response
            .data
            .ok_or_else(|| LoomClientError::ApiError("No data returned from API".to_string()))
    }

    // ========================================================================
    // Pinned Memory Operations
    // ========================================================================

    /// Get all pinned memories
    #[instrument(skip(self))]
    pub async fn get_pinned_memories(&self) -> Result<PinnedMemoriesResponse, LoomClientError> {
        let url = format!("{}/api/v1/pinned-memories", self.base_url);
        debug!("Getting pinned memories from: {}", url);

        let response = self.client.get(&url).send().await?;
        let api_response: ApiResponse<PinnedMemoriesResponse> =
            Self::handle_response(response).await?;

        api_response
            .data
            .ok_or_else(|| LoomClientError::ApiError("No data returned from API".to_string()))
    }

    /// Pin a memory by ID
    #[instrument(skip(self))]
    pub async fn pin_memory(
        &self,
        memory_id: i64,
        reason: Option<String>,
    ) -> Result<PinnedMemory, LoomClientError> {
        let url = format!("{}/api/v1/pinned-memories", self.base_url);
        debug!("Pinning memory {} at: {}", memory_id, url);

        let request = PinMemoryRequest { memory_id, reason };
        let response = self.client.post(&url).json(&request).send().await?;
        Self::handle_response(response).await
    }

    /// Unpin a memory by ID
    #[instrument(skip(self))]
    pub async fn unpin_memory(&self, memory_id: i64) -> Result<(), LoomClientError> {
        let url = format!("{}/api/v1/pinned-memories/{}", self.base_url, memory_id);
        debug!("Unpinning memory {} at: {}", memory_id, url);

        let response = self.client.delete(&url).send().await?;
        let status = response.status();

        if status.is_success() {
            Ok(())
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());
            Err(LoomClientError::ApiError(format!("HTTP {}: {}", status, error_text)))
        }
    }

    /// Get the base URL this client is configured to use
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[async_trait]
impl LoomClientTrait for LoomClient {
    async fn health_check(&self) -> Result<serde_json::Value, LoomClientError> {
        // Call the inherent method
        LoomClient::health_check(self).await
    }

    async fn create_memory(
        &self,
        request: CreateMemoryRequest,
    ) -> Result<MemoryResponse, LoomClientError> {
        LoomClient::create_memory(self, request).await
    }

    async fn create_single_memory(
        &self,
        fragment: MemoryFragment,
    ) -> Result<MemoryResponse, LoomClientError> {
        LoomClient::create_single_memory(self, fragment).await
    }

    async fn get_memory(&self, id: i64) -> Result<MemoryResponse, LoomClientError> {
        LoomClient::get_memory(self, id).await
    }

    async fn delete_memory(&self, id: i64) -> Result<(), LoomClientError> {
        LoomClient::delete_memory(self, id).await
    }

    async fn get_recent_memories(&self, limit: usize) -> Result<MemoryResponse, LoomClientError> {
        LoomClient::get_recent_memories(self, limit).await
    }

    async fn get_timeline_memory(
        &self,
        from: &str,
        to: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<MemoryResponse, LoomClientError> {
        LoomClient::get_timeline_memory(self, from, to, limit, offset).await
    }

    async fn get_pinned_memories(&self) -> Result<PinnedMemoriesResponse, LoomClientError> {
        LoomClient::get_pinned_memories(self).await
    }

    async fn pin_memory(
        &self,
        memory_id: i64,
        reason: Option<String>,
    ) -> Result<PinnedMemory, LoomClientError> {
        LoomClient::pin_memory(self, memory_id, reason).await
    }

    async fn unpin_memory(&self, memory_id: i64) -> Result<(), LoomClientError> {
        LoomClient::unpin_memory(self, memory_id).await
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
        let client = LoomClient::new("http://localhost:3000", http_client);
        assert_eq!(client.base_url(), "http://localhost:3000");
    }
}
