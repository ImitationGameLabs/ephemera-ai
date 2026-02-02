use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;
use tracing::info;
use time;

use loom::system_configs::models::{
    CreateSystemConfigRequest, SystemConfigQuery, SystemConfigResponse
};

/// System configs API client error
#[derive(Error, Debug)]
pub enum SystemConfigClientError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// System configs API client
pub struct SystemConfigClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl SystemConfigClient {
    /// Create a new system configs client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Build the full URL for an endpoint
    fn url(&self, path: &str) -> String {
        format!("{}/api/v1/system-configs{}", self.base_url, path)
    }

    /// Handle API response
    async fn handle_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, SystemConfigClientError> {
        let status = response.status();
        let text = response.text().await?;

        if status.is_success() {
            let api_response: ApiResponse<T> = serde_json::from_str(&text)?;

            if api_response.success {
                api_response.data.ok_or_else(|| {
                    SystemConfigClientError::ApiError("No data in successful response".to_string())
                })
            } else {
                Err(SystemConfigClientError::ApiError(
                    api_response.error.unwrap_or_else(|| "Unknown API error".to_string())
                ))
            }
        } else {
            Err(SystemConfigClientError::ApiError(format!(
                "HTTP {}: {}",
                status, text
            )))
        }
    }

    /// Create a system config record
    pub async fn create(&self, request: CreateSystemConfigRequest) -> Result<SystemConfigResponse, SystemConfigClientError> {
        info!("Creating system config record");

        let url = self.url("/");
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Query system config records
    pub async fn query(&self, query: SystemConfigQuery) -> Result<SystemConfigResponse, SystemConfigClientError> {
        info!("Querying system config records");

        let url = self.url("/");

        // Build query parameters
        let mut params = Vec::new();

        if let Some(memory_fragment_id) = query.memory_fragment_id {
            params.push(format!("memory_fragment_id={}", memory_fragment_id));
        }

        if let Some(content_hash) = &query.content_hash {
            params.push(format!("content_hash={}", content_hash));
        }

        if let Some(start_time) = &query.start_time {
            params.push(format!("start_time={}", start_time.format(&time::format_description::well_known::Iso8601::DEFAULT).unwrap()));
        }

        if let Some(end_time) = &query.end_time {
            params.push(format!("end_time={}", end_time.format(&time::format_description::well_known::Iso8601::DEFAULT).unwrap()));
        }

        if let Some(limit) = query.limit {
            params.push(format!("limit={}", limit));
        }

        if let Some(offset) = query.offset {
            params.push(format!("offset={}", offset));
        }

        let full_url = if params.is_empty() {
            url
        } else {
            format!("{}?{}", url, params.join("&"))
        };

        let response = self.client
            .get(&full_url)
            .send()
            .await?;

        self.handle_response(response).await
    }

    /// Get system config by ID
    pub async fn get_by_id(&self, id: i64) -> Result<SystemConfigResponse, SystemConfigClientError> {
        info!("Getting system config record with id: {}", id);

        let url = self.url(&format!("/{}", id));
        let response = self.client
            .get(&url)
            .send()
            .await?;

        self.handle_response(response).await
    }
}