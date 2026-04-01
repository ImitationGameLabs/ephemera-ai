use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub bio: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub bio: String,
    pub status: UserStatus,
    pub message_height: i32,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
}

/// Request body for updating the authenticated user's profile.
///
/// This endpoint is reserved for future expansion to include private user settings
/// (personal preferences) that only the user themselves can see and modify.
/// Currently returns the same data as the public profile API.
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub username: String,
    pub current_password: String,
    pub bio: Option<String>,
    pub new_password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OnlineStatus {
    pub online: bool,
    pub last_seen: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserStatus {
    pub online: bool,
    #[serde(with = "time::serde::iso8601::option")]
    pub last_seen: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsersList {
    pub users: Vec<User>,
}
