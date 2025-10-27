use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;

use crate::db::{MessageManager, UserManager, CreateMessageDto};
use crate::models::{
    CreateMessageRequest, Message, Messages,
    GetMessagesQuery
};

pub async fn create_message(
    State((message_manager, user_manager)): State<(MessageManager, UserManager)>,
    Json(request): Json<CreateMessageRequest>,
) -> Result<(StatusCode, Json<Message>), (StatusCode, Json<serde_json::Value>)> {
    // First authenticate user
    match user_manager.authenticate_by_credentials(&request.username, &request.password).await {
        Ok(user) => {
            let create_dto = CreateMessageDto {
                content: request.content,
                sender: user.name,
            };

            match message_manager.create_message(&create_dto).await {
                Ok(message) => {
                    let response = Message {
                        id: message.id,
                        content: message.content,
                        sender: message.sender,
                        created_at: message.created_at,
                    };
                    Ok((StatusCode::CREATED, Json(response)))
                }
                Err(e) => {
                    tracing::error!("Failed to create message: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to create message" })),
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
                    tracing::error!("Failed to authenticate user for message creation: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Authentication failed" })),
                    ))
                }
            }
        }
    }
}

pub async fn get_messages(
    State(manager): State<MessageManager>,
    Query(query): Query<GetMessagesQuery>,
) -> Result<Json<Messages>, (StatusCode, Json<serde_json::Value>)> {
    let sender_filter = query.sender.as_deref();
    let limit = query.limit;
    let offset = query.offset;

    match manager.get_messages(sender_filter, limit, offset).await {
        Ok(messages) => {
            let message_responses: Vec<Message> = messages
                .into_iter()
                .map(|msg| Message {
                    id: msg.id,
                    content: msg.content,
                    sender: msg.sender,
                    created_at: msg.created_at,
                })
                .collect();

            Ok(Json(Messages {
                messages: message_responses,
            }))
        }
        Err(e) => {
            tracing::error!("Failed to get messages: {:?}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to retrieve messages" })),
            ))
        }
    }
}

pub async fn get_message(
    State(manager): State<MessageManager>,
    Path(id): Path<i32>,
) -> Result<Json<Message>, (StatusCode, Json<serde_json::Value>)> {
    match manager.get_message(id).await {
        Ok(message) => {
            let response = Message {
                id: message.id,
                content: message.content,
                sender: message.sender,
                created_at: message.created_at,
            };
            Ok(Json(response))
        }
        Err(e) => {
            match e {
                crate::db::DbError::MessageNotFound(_) => Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "Message not found" })),
                )),
                _ => {
                    tracing::error!("Failed to get message: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to retrieve message" })),
                    ))
                }
            }
        }
    }
}

pub async fn delete_message(
    State(manager): State<MessageManager>,
    Path(id): Path<i32>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    match manager.delete_message(id).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            match e {
                crate::db::DbError::MessageNotFound(_) => Err((
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "Message not found" })),
                )),
                _ => {
                    tracing::error!("Failed to delete message: {:?}", e);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": "Failed to delete message" })),
                    ))
                }
            }
        }
    }
}