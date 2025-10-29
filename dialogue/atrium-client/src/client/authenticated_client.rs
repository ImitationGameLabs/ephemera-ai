use std::sync::Arc;
use tokio::sync::Mutex;
use atrium::models::*;
use super::raw_client::{RawClient, ClientError};

#[derive(Clone)]
pub struct AuthenticatedClient {
    raw_client: RawClient,
    credentials: UserCredentials,
    pub(crate) user_info: Arc<Mutex<Option<User>>>,
}

impl AuthenticatedClient {
    pub fn new(raw_client: RawClient, credentials: UserCredentials) -> Self {
        Self {
            raw_client,
            credentials,
            user_info: Arc::new(Mutex::new(None)),
        }
    }

    pub fn credentials(&self) -> &UserCredentials {
        &self.credentials
    }

    pub async fn login(&self) -> Result<User, ClientError> {
        // First try to get user profile to verify user exists
        let user = self.raw_client.get_user_profile(&self.credentials.username).await?;

        // Then verify credentials by sending heartbeat
        self.raw_client.send_heartbeat(self.credentials.clone()).await?;

        // Store user info
        *self.user_info.lock().await = Some(user.clone());
        Ok(user)
    }

    pub async fn login_or_register(&self, bio: String) -> Result<User, ClientError> {
        // Try to login first
        match self.login().await {
            Ok(user) => Ok(user),
            Err(ClientError::ApiError(msg)) if msg.contains("404") || msg.contains("not found") => {
                // User doesn't exist, register them
                let register_request = CreateUserRequest {
                    name: self.credentials.username.clone(),
                    bio,
                    password: self.credentials.password.clone(),
                };

                let user = self.raw_client.register_user(register_request).await?;

                // Store user info after successful registration
                *self.user_info.lock().await = Some(user.clone());
                Ok(user)
            }
            Err(e) => Err(e),
        }
    }

    // New constructor that takes URL directly
    pub fn new_with_url(server_url: impl Into<String>, credentials: UserCredentials) -> Self {
        let raw_client = RawClient::new(server_url);
        Self {
            raw_client,
            credentials,
            user_info: Arc::new(Mutex::new(None)),
        }
    }

    // Login with existing user
    pub async fn connect_and_login(server_url: impl Into<String>, username: String, password: String) -> Result<Self, ClientError> {
        let credentials = UserCredentials { username, password };
        let client = Self::new_with_url(server_url, credentials);
        client.login().await?;
        Ok(client)
    }

    // Login or register with bio
    pub async fn connect_and_login_or_register(server_url: impl Into<String>, username: String, password: String, bio: String) -> Result<Self, ClientError> {
        let credentials = UserCredentials { username, password };
        let client = Self::new_with_url(server_url, credentials);
        client.login_or_register(bio).await?;
        Ok(client)
    }

    pub async fn user(&self) -> Option<User> {
        self.user_info.lock().await.clone()
    }

    pub async fn set_user(&self, user: User) {
        *self.user_info.lock().await = Some(user);
    }

    pub async fn send_message(&self, content: String) -> Result<Message, ClientError> {
        self.raw_client.send_message(
            &self.credentials.username,
            &self.credentials.password,
            content
        ).await
    }

    pub async fn send_heartbeat(&self) -> Result<(), ClientError> {
        self.raw_client.send_heartbeat(self.credentials.clone()).await
    }

    pub async fn get_messages(&self, limit: Option<u64>, offset: Option<u64>) -> Result<Messages, ClientError> {
        self.raw_client.get_messages(limit, offset).await
    }

    pub async fn get_all_users(&self) -> Result<UsersList, ClientError> {
        self.raw_client.get_all_users().await
    }

    pub async fn get_user_profile(&self, username: &str) -> Result<User, ClientError> {
        self.raw_client.get_user_profile(username).await
    }

    // Delegate to RawClient for other operations
    pub fn raw_client(&self) -> &RawClient {
        &self.raw_client
    }
}