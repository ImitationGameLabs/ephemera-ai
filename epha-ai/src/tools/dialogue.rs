use std::sync::Arc;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::json;
use atrium_client::{AuthenticatedClient, Message};
use atrium_client::ClientError as DialogueClientError;

#[derive(Deserialize)]
pub struct GetMessagesArgs {
    pub sender: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Deserialize)]
pub struct SendMessageArgs {
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Dialogue error: {0}")]
pub struct DialogueError(#[from] DialogueClientError);

pub struct GetMessages {
    dialogue_client: Arc<AuthenticatedClient>,
}

impl GetMessages {
    pub fn new(dialogue_client: Arc<AuthenticatedClient>) -> Self {
        Self { dialogue_client }
    }
}

impl Tool for GetMessages {
    const NAME: &'static str = "get_messages";

    type Error = DialogueError;
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
        let messages = self.dialogue_client.get_messages(args.limit, args.offset)
            .await?;

        // Filter by sender if specified
        let filtered_messages: Vec<&Message> = if let Some(sender) = args.sender {
            messages.messages.iter()
                .filter(|msg| msg.sender == sender)
                .collect()
        } else {
            messages.messages.iter().collect()
        };

        if filtered_messages.is_empty() {
            Ok("No messages found matching the criteria.".to_string())
        } else {
            let formatted_messages: Vec<String> = filtered_messages.iter()
                .map(|msg| {
                    format!("[{}] {}: {}", msg.created_at, msg.sender, msg.content)
                })
                .collect();

            Ok(format!("Retrieved {} messages:\n\n{}",
                filtered_messages.len(),
                formatted_messages.join("\n")))
        }
    }
}

pub struct SendMessage {
    dialogue_client: Arc<AuthenticatedClient>,
}

impl SendMessage {
    pub fn new(dialogue_client: Arc<AuthenticatedClient>) -> Self {
        Self { dialogue_client }
    }
}

impl Tool for SendMessage {
    const NAME: &'static str = "send_message";

    type Error = DialogueError;
    type Args = SendMessageArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "send_message",
            "description": "Send message to Dialogue Atrium",
            "parameters": {
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "The message to send"
                    }
                },
                "required": ["message"]
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let message = self.dialogue_client.send_message(args.message.clone()).await?;

        Ok(format!("Message sent successfully! ID: {}, Sent at: {}",
            message.id,
            message.created_at))
    }
}