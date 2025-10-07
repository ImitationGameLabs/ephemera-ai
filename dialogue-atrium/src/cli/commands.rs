use std::fmt;
use crate::cli::client::DialogueClient;
use crate::cli::auth::AuthSession;
use crate::models::UserResponse;

#[derive(Debug)]
pub enum CommandError {
    ParseError(String),
    ExecutionError(String),
    UnknownCommand(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::ParseError(msg) => write!(f, "Command parse error: {}", msg),
            CommandError::ExecutionError(msg) => write!(f, "Command execution error: {}", msg),
            CommandError::UnknownCommand(cmd) => write!(f, "Unknown command: {}. Use /help for available commands", cmd),
        }
    }
}

pub struct CommandContext {
    pub client: DialogueClient,
    pub session: AuthSession,
}

pub struct CommandHandler {
    pub ctx: CommandContext,
}

impl CommandHandler {
    pub fn new(ctx: CommandContext) -> Self {
        Self { ctx }
    }

    pub fn parse_and_execute(&mut self, input: &str) -> Result<Option<String>, CommandError> {
        if !input.starts_with('/') {
            return Ok(None); // Not a command
        }

        let parts: Vec<&str> = input.trim_start_matches('/').split_whitespace().collect();
        if parts.is_empty() {
            return Err(CommandError::ParseError("Empty command".to_string()));
        }

        let command = parts[0];
        let args = &parts[1..];

        match command {
            "help" => Ok(Some(self.cmd_help(args)?)),
            "list-active-users" => Ok(Some(self.cmd_list_active_users(args)?)),
            "list-users" => Ok(Some(self.cmd_list_users(args)?)),
            "profile" => Ok(Some(self.cmd_profile(args)?)),
            "history" => Ok(Some(self.cmd_history(args)?)),
            "clear" => Ok(Some(self.cmd_clear(args)?)),
            "exit" | "quit" => Ok(Some(self.cmd_exit(args)?)),
            _ => Err(CommandError::UnknownCommand(command.to_string())),
        }
    }

    fn cmd_help(&self, _args: &[&str]) -> Result<String, CommandError> {
        let help_text = r#"
Available Commands:
  /help                    - Show this help message
  /list-active-users      - List all active (online) users
  /list-users             - List all registered users
  /profile [username]     - Show profile for username or yourself if not provided
  /history [count]        - Show message history (default 20, max 100 messages)
  /clear                  - Clear the screen
  /exit or /quit          - Exit the CLI client

Chat:
  Type any message (without '/') to send it to the chat.
  Messages will appear in real-time from other users.
        "#.trim();

        Ok(help_text.to_string())
    }

  
    fn cmd_list_active_users(&mut self, _args: &[&str]) -> Result<String, CommandError> {
        let client = self.ctx.client.clone();

        tokio::spawn(async move {
            match client.get_all_users().await {
                Ok(users_response) => {
                    let active_users: Vec<&UserResponse> = users_response.users
                        .iter()
                        .filter(|user| user.status.online)
                        .collect();

                    if active_users.is_empty() {
                        println!("No active users found.");
                    } else {
                        println!("Active Users ({}):", active_users.len());
                        for user in active_users {
                            let last_seen = match user.status.last_seen {
                                Some(time) => format!("Last seen: {}", time),
                                None => "Currently online".to_string(),
                            };
                            println!("  • {} - {} ({})", user.name, user.bio, last_seen);
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to fetch active users: {}", e);
                }
            }
        });

        Ok("Fetching active users...".to_string())
    }

    fn cmd_list_users(&mut self, _args: &[&str]) -> Result<String, CommandError> {
        let client = self.ctx.client.clone();

        tokio::spawn(async move {
            match client.get_all_users().await {
                Ok(users_response) => {
                    if users_response.users.is_empty() {
                        println!("No users found.");
                    } else {
                        println!("All Users ({}):", users_response.users.len());
                        for user in users_response.users {
                            let status = if user.status.online {
                                "Online".to_string()
                            } else {
                                match user.status.last_seen {
                                    Some(time) => format!("Offline (last seen: {})", time),
                                    None => "Offline".to_string(),
                                }
                            };
                            println!("  • {} - {} [{}]", user.name, user.bio, status);
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to fetch users: {}", e);
                }
            }
        });

        Ok("Fetching all users...".to_string())
    }

    fn cmd_profile(&mut self, args: &[&str]) -> Result<String, CommandError> {
        let username = if args.is_empty() {
            &self.ctx.session.username
        } else {
            args[0]
        };

        let client = self.ctx.client.clone();
        let username_str = username.to_string();
        let display_username = username_str.clone();

        tokio::spawn(async move {
            match client.get_user_profile(&username_str).await {
                Ok(user) => {
                    println!("Profile for {}:", user.name);
                    println!("  Bio: {}", user.bio);
                    println!("  Status: {}", if user.status.online { "Online" } else { "Offline" });
                    if let Some(last_seen) = user.status.last_seen {
                        println!("  Last Seen: {}", last_seen);
                    }
                    println!("  Message Height: {}", user.message_height);
                    println!("  Created At: {}", user.created_at);
                }
                Err(e) => {
                    println!("Failed to fetch profile for '{}': {}", username_str, e);
                }
            }
        });

        Ok(format!("Fetching profile for {}...", display_username))
    }

    fn cmd_history(&mut self, args: &[&str]) -> Result<String, CommandError> {
        let count = if args.is_empty() {
            20u64 // Default to 20 messages
        } else {
            match args[0].parse::<u64>() {
                Ok(n) => n.min(100), // Cap at 100 messages
                Err(_) => return Err(CommandError::ParseError("Invalid number for history count".to_string())),
            }
        };

        let client = self.ctx.client.clone();

        tokio::spawn(async move {
            match client.get_messages(Some(count), None).await {
                Ok(messages_response) => {
                    if messages_response.messages.is_empty() {
                        println!("No messages found.");
                    } else {
                        println!("Recent messages ({}):", messages_response.messages.len());
                        for message in messages_response.messages {
                            // Simple time formatting - just use the raw datetime format
                        let time_str = format!("{}", message.created_at);
                            println!("[{}] {}: {}", time_str, message.sender, message.content);
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to fetch message history: {}", e);
                }
            }
        });

        Ok(format!("Fetching last {} messages...", count))
    }

    fn cmd_clear(&self, _args: &[&str]) -> Result<String, CommandError> {
        // Clear screen by printing 50 newlines
        for _ in 0..50 {
            println!();
        }
        Ok("Screen cleared.".to_string())
    }

    fn cmd_exit(&self, _args: &[&str]) -> Result<String, CommandError> {
        Ok("Goodbye!".to_string())
    }

    pub async fn send_message(&mut self, content: String) -> Result<String, CommandError> {
        let client = self.ctx.client.clone();
        let username = self.ctx.session.username.clone();
        let password = self.ctx.session.password.clone();

        match client.send_message(&username, &password, content).await {
            Ok(message) => {
                Ok(format!("Message sent: {}", message.content))
            }
            Err(e) => {
                Err(CommandError::ExecutionError(format!("Failed to send message: {}", e)))
            }
        }
    }
}