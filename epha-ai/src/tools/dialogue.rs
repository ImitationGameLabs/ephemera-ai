use std::sync::Arc;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::Deserialize;
use serde_json::json;
use atrium_client::{AuthenticatedClient, UnreadMessages};
use atrium_client::ClientError as DialogueClientError;

const GET_MESSAGES_LIMIT: u64 = 20;

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
    type Args = ();
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "get_messages",
            "description": "Get unread messages from Dialogue Atrium"
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let unread_result: UnreadMessages = self.dialogue_client.get_unread_messages(Some(GET_MESSAGES_LIMIT)).await?;

        if unread_result.messages.is_empty() {
            if unread_result.remaining_unread > 0 {
                Ok(format!("No new messages retrieved. {} messages remain unread in total.", unread_result.remaining_unread))
            } else {
                Ok("No unread messages found. You're all caught up!".to_string())
            }
        } else {
            let formatted_messages: Vec<String> = unread_result.messages.iter()
                .map(|msg| {
                    format!("[{}] {}: {}", msg.created_at, msg.sender, msg.content)
                })
                .collect();

            let remaining_text = if unread_result.remaining_unread > 0 {
                format!("{} more messages remain unread.", unread_result.remaining_unread)
            } else {
                "No more unread messages.".to_string()
            };

            Ok(format!("Retrieved {} unread messages:\n\n{}\n\n{}",
                unread_result.messages.len(),
                formatted_messages.join("\n"),
                remaining_text))
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