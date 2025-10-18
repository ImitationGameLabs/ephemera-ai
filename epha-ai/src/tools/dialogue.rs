use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct GetMessagesArgs {
    pub room_id: String,
}

#[derive(Deserialize)]
pub struct SendMessageArgs {
    pub room_id: String,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Dialogue Atrium error")]
pub struct DialogueAtriumError;

#[derive(Deserialize, Serialize)]
pub struct GetMessages;

impl Tool for GetMessages {
    const NAME: &'static str = "get_messages";

    type Error = DialogueAtriumError;
    type Args = GetMessagesArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "get_messages",
            "description": "Get unread messages from Dialogue Atrium room",
            "parameters": {
                "type": "object",
                "properties": {
                    "room_id": {
                        "type": "string",
                        "description": "The ID of the room to get messages from"
                    }
                },
                "required": ["room_id"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // TODO: Implement actual Dialogue Atrium client
        Ok(format!("Retrieved messages from room {}", args.room_id))
    }
}

#[derive(Deserialize, Serialize)]
pub struct SendMessage;

impl Tool for SendMessage {
    const NAME: &'static str = "send_message";

    type Error = DialogueAtriumError;
    type Args = SendMessageArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "send_message",
            "description": "Send message to Dialogue Atrium room",
            "parameters": {
                "type": "object",
                "properties": {
                    "room_id": {
                        "type": "string",
                        "description": "The ID of the room to send message to"
                    },
                    "message": {
                        "type": "string",
                        "description": "The message to send"
                    }
                },
                "required": ["room_id", "message"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // TODO: Implement actual Dialogue Atrium client
        Ok(format!("Sent message '{}' to room {}", args.message, args.room_id))
    }
}