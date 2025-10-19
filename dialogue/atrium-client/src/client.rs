use reqwest::{Client, Response, Error as ReqwestError};
use serde::de::DeserializeOwned;
use std::fmt;
use atrium::models::{CreateUserRequest, UserCredentials, UserResponse, UsersListResponse};
use atrium::models::{CreateMessageRequest, MessageResponse, MessagesResponse};

#[derive(Debug)]
pub enum ClientError {
    NetworkError(ReqwestError),
    ApiError(String),
    JsonError(serde_json::Error),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientError::NetworkError(e) => write!(f, "Network error: {}", e),
            ClientError::ApiError(msg) => write!(f, "API error: {}", msg),
            ClientError::JsonError(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl From<ReqwestError> for ClientError {
    fn from(error: ReqwestError) -> Self {
        ClientError::NetworkError(error)
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(error: serde_json::Error) -> Self {
        ClientError::JsonError(error)
    }
}

#[derive(Clone)]
pub struct DialogueClient {
    client: Client,
    base_url: String,
}

impl DialogueClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }

    async fn handle_response<T: DeserializeOwned>(response: Response) -> Result<T, ClientError> {
        let status = response.status();

        if status.is_success() {
            let text = response.text().await?;
            serde_json::from_str(&text).map_err(Into::into)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(ClientError::ApiError(format!("{}: {}", status, error_text)))
        }
    }

    pub async fn register_user(&self, request: CreateUserRequest) -> Result<UserResponse, ClientError> {
        let response = self.client
            .post(format!("{}/api/v1/users", self.base_url))
            .json(&request)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn authenticate(&self, username: &str, password: &str) -> Result<UserResponse, ClientError> {
        // First check if user exists
        match self.get_user_profile(username).await {
            Ok(user) => {
                // User exists, now verify password with heartbeat
                if self.send_heartbeat(UserCredentials {
                    username: username.to_string(),
                    password: password.to_string()
                }).await.is_ok() {
                    Ok(user)
                } else {
                    Err(ClientError::ApiError("Invalid password".to_string()))
                }
            }
            Err(e) => {
                // User doesn't exist or other error
                Err(e)
            }
        }
    }

    pub async fn get_all_users(&self) -> Result<UsersListResponse, ClientError> {
        let response = self.client
            .get(format!("{}/api/v1/users", self.base_url))
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn get_user_profile(&self, username: &str) -> Result<UserResponse, ClientError> {
        let response = self.client
            .get(format!("{}/api/v1/users/{}", self.base_url, username))
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn send_heartbeat(&self, credentials: UserCredentials) -> Result<(), ClientError> {
        let response = self.client
            .post(format!("{}/api/v1/heartbeat", self.base_url))
            .json(&credentials)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(ClientError::ApiError(format!("{}: {}", status, error_text)))
        }
    }

    pub async fn send_message(&self, username: &str, password: &str, content: String) -> Result<MessageResponse, ClientError> {
        let request = CreateMessageRequest {
            content,
            username: username.to_string(),
            password: password.to_string(),
        };

        let response = self.client
            .post(format!("{}/api/v1/messages", self.base_url))
            .json(&request)
            .send()
            .await?;

        Self::handle_response(response).await
    }

    pub async fn get_messages(&self, limit: Option<u64>, offset: Option<u64>) -> Result<MessagesResponse, ClientError> {
        let mut query_params = Vec::new();

        if let Some(limit) = limit {
            query_params.push(format!("limit={}", limit));
        }

        if let Some(offset) = offset {
            query_params.push(format!("offset={}", offset));
        }

        let url = if query_params.is_empty() {
            format!("{}/api/v1/messages", self.base_url)
        } else {
            format!("{}/api/v1/messages?{}", self.base_url, query_params.join("&"))
        };

        let response = self.client
            .get(&url)
            .send()
            .await?;

        Self::handle_response(response).await
    }
}