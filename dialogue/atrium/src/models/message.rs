use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: i32,
    pub content: String,
    pub sender: String,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize)]
pub struct GetMessagesQuery {
    pub sender: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessagesResponse {
    pub messages: Vec<MessageResponse>,
}