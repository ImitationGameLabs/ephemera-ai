use axum::{
    extract::{State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;

use crate::db::{
    user_manager::{UserManager, UserError},
    message_manager::MessageManager,
};
use crate::models::{UserCredentials, OnlineStatus};

pub async fn update_heartbeat(
    State((user_manager, message_manager)): State<(UserManager, MessageManager)>,
    Json(request): Json<UserCredentials>,
) -> Result<(StatusCode, Json<OnlineStatus>), (StatusCode, Json<serde_json::Value>)> {
    // First authenticate user
    match user_manager.authenticate_by_credentials(&request.username, &request.password).await {
        Ok(user) => {
            // Update heartbeat
            match user_manager.update_heartbeat(&user.name).await {
                Ok(updated_user) => {
                    // Get latest message ID to potentially update user's message height
                    if let Ok(Some(latest_message_id)) = message_manager.get_latest_message_id().await {
                        // Update user's message height to the latest message ID
                        let _ = user_manager.update_message_height(&user.name, latest_message_id).await;
                    }

                    let response = OnlineStatus {
                        online: true,
                        last_seen: updated_user.status.last_seen,
                    };
                    Ok((StatusCode::OK, Json(response)))
                }
                Err(e) => {
                    tracing::error!("Failed to update heartbeat: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to update heartbeat" })),
                    ))
                }
            }
        }
        Err(e) => {
            match e {
                UserError::InvalidPassword(_) => Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "Invalid password" })),
                )),
                _ => {
                    tracing::error!("Failed to authenticate user for heartbeat: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Authentication failed" })),
                    ))
                }
            }
        }
    }
}