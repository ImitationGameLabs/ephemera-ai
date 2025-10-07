use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub bio: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserResponse {
    pub name: String,
    pub bio: String,
    pub status: UserStatus,
    pub message_height: i32,
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub current_password: String,
    pub bio: Option<String>,
    pub new_password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordAuth {
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OnlineStatus {
    pub online: bool,
    pub last_seen: Option<PrimitiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserStatus {
    pub online: bool,
    pub last_seen: Option<PrimitiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsersListResponse {
    pub users: Vec<UserResponse>,
}