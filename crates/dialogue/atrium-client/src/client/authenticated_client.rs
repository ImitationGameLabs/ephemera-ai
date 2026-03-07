use std::sync::Arc;
use tokio::sync::Mutex;
use atrium::models::*;
use super::raw_client::{RawClient, ClientError};

#[derive(Clone)]
pub struct AuthenticatedClient {
    raw_client: RawClient,
    credentials: UserCredentials,
    last_read_message_id: Arc<Mutex<Option<i32>>>,
}

impl AuthenticatedClient {
    pub fn new(raw_client: RawClient, credentials: UserCredentials) -> Self {
        Self {
            raw_client,
            credentials,
            last_read_message_id: Arc::new(Mutex::new(None)),
        }
    }

    pub fn credentials(&self) -> &UserCredentials {
        &self.credentials
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

    pub async fn login(&self) -> Result<User, ClientError> {
        // First try to get user profile to verify user exists
        let user = self.raw_client.get_user_profile(&self.credentials.username).await?;

        // Then verify credentials by sending heartbeat
        self.raw_client.send_heartbeat(self.credentials.clone()).await?;

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
            last_read_message_id: Arc::new(Mutex::new(None)),
        }
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

    pub async fn get_messages(&self, query: GetMessagesQuery) -> Result<Messages, ClientError> {
        self.raw_client.get_messages(query).await
    }

    pub async fn get_all_users(&self) -> Result<UsersList, ClientError> {
        self.raw_client.get_all_users().await
    }

    pub async fn get_user_profile(&self, username: &str) -> Result<User, ClientError> {
        self.raw_client.get_user_profile(username).await
    }

    // Get unread messages for the current user
    pub async fn get_unread_messages(&self, limit: Option<u64>) -> Result<UnreadMessages, ClientError> {
        // First, check if we need to initialize last_read_message_id
        let initialization_needed = self.last_read_message_id.lock().await.is_none();

        if initialization_needed {
            // Fetch user profile directly from server to get message_height
            let user = self.raw_client.get_user_profile(&self.credentials.username).await?;
            let message_height = user.message_height;

            // Set the last_read_message_id
            let _ = self.last_read_message_id.lock().await.insert(message_height);
        }

        // Get the current since_id
        let since_id = self.last_read_message_id.lock().await.clone();

        // Fetch messages using since_id
        let query = GetMessagesQuery {
            sender: None,
            limit,
            offset: None,
            since_id,
        };

        let messages_response = self.raw_client.get_messages(query).await?;

        // Update last_read_message_id to the highest message ID we received
        if let Some(highest_id) = messages_response.messages.iter().map(|m| m.id).max() {
            let _ = self.last_read_message_id.lock().await.insert(highest_id);
        }

        // Calculate remaining unread messages
        let remaining_unread = self.calculate_remaining_unread().await?;

        Ok(UnreadMessages {
            messages: messages_response.messages,
            remaining_unread,
        })
    }

    // Helper method to calculate remaining unread messages
    async fn calculate_remaining_unread(&self) -> Result<i64, ClientError> {
        // Get the latest message ID in the system
        let latest_messages = self.raw_client.get_messages(GetMessagesQuery {
            sender: None,
            limit: Some(1),
            offset: None,
            since_id: None,
        }).await?;

        // Get the last_read_id after the HTTP call
        let last_read_id = self.last_read_message_id.lock().await.clone();

        if latest_messages.messages.is_empty() {
            return Ok(0)
        }

        let latest_message = latest_messages.messages.first().unwrap();
        if let Some(last_read) = last_read_id {
            Ok((latest_message.id - last_read) as i64)
        } else {
            // If no last_read_id, count all messages as unread
            // This is a fallback - ideally we'd get the total count
            Ok(latest_messages.messages.len() as i64)
        }
    }

    // Delegate to RawClient for other operations
    pub fn raw_client(&self) -> &RawClient {
        &self.raw_client
    }
}