use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
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
    pub since_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Messages {
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnreadMessages {
    pub messages: Vec<Message>,
    pub remaining_unread: i64,
}