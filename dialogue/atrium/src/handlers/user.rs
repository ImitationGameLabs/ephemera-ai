use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;

use crate::db::{UserManager, CreateUserDto, UpdateUserDto};
use crate::models::{
    CreateUserRequest, User, UpdateProfileRequest,
    PasswordAuth, UserStatus, UsersList
};

pub async fn create_user(
    State(user_manager): State<UserManager>,
    Json(request): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), (StatusCode, Json<serde_json::Value>)> {
    let create_dto = CreateUserDto {
        name: request.name.clone(),
        bio: request.bio.clone(),
        password: request.password,
    };

    match user_manager.create_user(&create_dto).await {
        Ok(user) => {
            let response = User {
                name: user.name,
                bio: user.bio,
                status: UserStatus {
                    online: user.online,
                    last_seen: user.last_seen,
                },
                message_height: user.message_height,
                created_at: user.created_at,
            };
            Ok((StatusCode::CREATED, Json(response)))
        }
        Err(e) => {
            match e {
                crate::db::UserError::UserAlreadyExists(name) => Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("User '{}' already exists", name) })),
                )),
                _ => {
                    tracing::error!("Failed to create user: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to create user" })),
                    ))
                }
            }
        }
    }
}

pub async fn get_user_profile(
    State(user_manager): State<UserManager>,
    Path(username): Path<String>,
) -> Result<Json<User>, (StatusCode, Json<serde_json::Value>)> {
    match user_manager.get_user_by_name(&username).await {
        Ok(user) => {
            let response = User {
                name: user.name,
                bio: user.bio,
                status: UserStatus {
                    online: user.online,
                    last_seen: user.last_seen,
                },
                message_height: user.message_height,
                created_at: user.created_at,
            };
            Ok(Json(response))
        }
        Err(e) => {
            match e {
                crate::db::UserError::UserNotFound(_) => Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "User not found" })),
                )),
                _ => {
                    tracing::error!("Failed to get user profile: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to retrieve user profile" })),
                    ))
                }
            }
        }
    }
}

pub async fn get_own_profile(
    State(user_manager): State<UserManager>,
    Json(request): Json<PasswordAuth>,
) -> Result<Json<User>, (StatusCode, Json<serde_json::Value>)> {
    // For now, we'll need to determine the username from the password
    // This is a simplified approach - in a real system you might want to include username in auth
    match user_manager.authenticate_user(&request.password, &request.password).await {
        Ok(user) => {
            let response = User {
                name: user.name,
                bio: user.bio,
                status: UserStatus {
                    online: user.online,
                    last_seen: user.last_seen,
                },
                message_height: user.message_height,
                created_at: user.created_at,
            };
            Ok(Json(response))
        }
        Err(e) => {
            match e {
                crate::db::UserError::InvalidPassword(_) => Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "Invalid password" })),
                )),
                _ => {
                    tracing::error!("Failed to authenticate user: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Authentication failed" })),
                    ))
                }
            }
        }
    }
}

pub async fn update_profile(
    State(user_manager): State<UserManager>,
    Json(request): Json<UpdateProfileRequest>,
) -> Result<Json<User>, (StatusCode, Json<serde_json::Value>)> {
    // First authenticate with current password
    match user_manager.authenticate_user(&request.current_password, &request.current_password).await {
        Ok(user) => {
            let update_dto = UpdateUserDto {
                bio: request.bio,
                new_password: request.new_password,
            };

            match user_manager.update_user(&user.name, &update_dto).await {
                Ok(updated_user) => {
                    let response = User {
                        name: updated_user.name,
                        bio: updated_user.bio,
                        status: UserStatus {
                            online: updated_user.online,
                            last_seen: updated_user.last_seen,
                        },
                        message_height: updated_user.message_height,
                        created_at: updated_user.created_at,
                    };
                    Ok(Json(response))
                }
                Err(e) => {
                    tracing::error!("Failed to update user profile: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to update profile" })),
                    ))
                }
            }
        }
        Err(e) => {
            match e {
                crate::db::UserError::InvalidPassword(_) => Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "Invalid password" })),
                )),
                _ => {
                    tracing::error!("Failed to authenticate user for profile update: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Authentication failed" })),
                    ))
                }
            }
        }
    }
}

pub async fn get_all_users(
    State(user_manager): State<UserManager>,
) -> Result<Json<UsersList>, (StatusCode, Json<serde_json::Value>)> {
    match user_manager.get_all_users().await {
        Ok(users) => {
            let user_responses: Vec<User> = users
                .into_iter()
                .map(|user| User {
                    name: user.name,
                    bio: user.bio,
                    status: UserStatus {
                        online: user.online,
                        last_seen: user.last_seen,
                    },
                    message_height: user.message_height,
                    created_at: user.created_at,
                })
                .collect();

            Ok(Json(UsersList {
                users: user_responses,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to get all users: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to retrieve users" })),
            ))
        }
    }
}