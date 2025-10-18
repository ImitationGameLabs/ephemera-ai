use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct GetMessagesArgs {
    pub sender: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Deserialize)]
pub struct SendMessageArgs {
    pub username: String,
    pub password: String,
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
            "description": "Get messages from Dialogue Atrium",
            "parameters": {
                "type": "object",
                "properties": {
                    "sender": {
                        "type": "string",
                        "description": "Optional filter to get messages only from specific sender"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of messages to retrieve"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Number of messages to skip for pagination"
                    }
                },
                "required": []
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // TODO: Implement actual Dialogue Atrium client
        let sender_desc = args.sender.map(|s| format!("from sender: {}", s)).unwrap_or_else(|| "from all senders".to_string());
        let limit_desc = args.limit.map(|l| format!("limit: {}", l)).unwrap_or_else(|| "no limit".to_string());
        Ok(format!("Retrieved messages {} ({}, {})", sender_desc, limit_desc,
                   args.offset.map(|o| format!("offset: {}", o)).unwrap_or_else(|| "no offset".to_string())))
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
            "description": "Send message to Dialogue Atrium",
            "parameters": {
                "type": "object",
                "properties": {
                    "username": {
                        "type": "string",
                        "description": "Your username for authentication"
                    },
                    "password": {
                        "type": "string",
                        "description": "Your password for authentication"
                    },
                    "message": {
                        "type": "string",
                        "description": "The message to send"
                    }
                },
                "required": ["username", "password", "message"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // TODO: Implement actual Dialogue Atrium client
        Ok(format!("Sent message '{}' as user '{}'", args.message, args.username))
    }
}