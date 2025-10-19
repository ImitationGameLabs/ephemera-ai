use std::io::{self, Write};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;
use tokio::time::interval;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use atrium_client::{DialogueClient, AuthManager, AuthSession};
use crate::commands::{CommandHandler, CommandContext, CommandError};
use atrium::models::UserCredentials;

pub struct CliInterface {
    client: DialogueClient,
    auth_manager: AuthManager,
    heartbeat_sender: Option<mpsc::Sender<()>>,
    heartbeat_handle: Option<JoinHandle<()>>,
    message_receiver_handle: Option<JoinHandle<()>>,
    last_message_id: std::sync::atomic::AtomicI32,
}

impl Drop for CliInterface {
    fn drop(&mut self) {
        // Stop heartbeat when dropping
        if let Some(sender) = self.heartbeat_sender.take() {
            let _ = sender.try_send(());
        }
        if let Some(handle) = self.heartbeat_handle.take() {
            handle.abort();
        }
        if let Some(handle) = self.message_receiver_handle.take() {
            handle.abort();
        }
    }
}

impl CliInterface {
    pub fn new(client: DialogueClient) -> Self {
        let auth_manager = AuthManager::new(client.clone());

        Self {
            client,
            auth_manager,
            heartbeat_sender: None,
            heartbeat_handle: None,
            message_receiver_handle: None,
            last_message_id: AtomicI32::new(0),
        }
    }

    fn start_heartbeat_task(&mut self, username: String, password: String) {
        let (sender, mut receiver) = mpsc::channel::<()>(1);
        let client = self.client.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Send heartbeat every 30 seconds

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = client.send_heartbeat(UserCredentials {
                            username: username.clone(),
                            password: password.clone()
                        }).await {
                            eprintln!("Heartbeat failed: {}", e);
                        }
                    }
                    _ = receiver.recv() => {
                        // Stop signal received
                        break;
                    }
                }
            }
        });

        self.heartbeat_sender = Some(sender);
        self.heartbeat_handle = Some(handle);
    }

    fn start_message_receiver_task(&mut self) {
        // For now, we'll use a simple approach without background message polling
        // This can be implemented later with proper synchronization
    }

    async fn load_initial_history(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.client.get_messages(Some(20), None).await {
            Ok(messages_response) => {
                if !messages_response.messages.is_empty() {
                    println!("Recent messages ({}):", messages_response.messages.len());
                    for message in messages_response.messages {
                        // Simple time formatting - just use the raw datetime format
                        let time_str = format!("{}", message.created_at);
                        println!("[{}] {}: {}", time_str, message.sender, message.content);

                        // Update last message ID
                        if message.id > self.last_message_id.load(Ordering::Relaxed) {
                            self.last_message_id.store(message.id, Ordering::Relaxed);
                        }
                    }
                    println!(); // Add empty line for separation
                } else {
                    println!("No recent messages found.");
                }
            }
            Err(e) => {
                return Err(format!("Failed to load message history: {}", e).into());
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Authenticate or register user
        let session = self.auth_manager.authenticate_or_register().await?;

        self.run_with_session(session).await
    }

    pub async fn run_with_credentials(&mut self, user: Option<String>, password: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let session = match (user, password) {
            (Some(username), Some(password)) => {
                // Use provided credentials
                self.auth_manager.authenticate_or_register_with_credentials(username, password).await?
            }
            _ => {
                // Fall back to interactive mode
                self.auth_manager.authenticate_or_register().await?
            }
        };

        self.run_with_session(session).await
    }

    async fn run_with_session(&mut self, session: AuthSession) -> Result<(), Box<dyn std::error::Error>> {
        // Start heartbeat in background
        self.start_heartbeat_task(session.username.clone(), session.password.clone());
        println!("✓ Heartbeat started - you will appear as online");

        // Load initial message history
        println!("Loading recent message history...");
        if let Err(e) = self.load_initial_history().await {
            println!("Failed to load message history: {}", e);
        }

        // Start message receiver
        self.start_message_receiver_task();
        println!("✓ Message receiver started - you will see real-time messages");

        // Create command handler
        let ctx = CommandContext {
            client: self.client.clone(),
            session,
        };
        let mut command_handler = CommandHandler::new(ctx);

        println!("\nWelcome to Dialogue Atrium CLI! 🎉");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("💬 Type any message to chat (no '/' needed)");
        println!("📋 Type '/help' for available commands");
        println!("🚪 Type '/exit' to quit");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // Main command loop
        loop {
            print!("{}> ", command_handler.ctx.session.username);
            io::stdout().flush().unwrap();

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => {
                    // EOF (Ctrl+D)
                    println!("\nGoodbye!");
                    break;
                }
                Ok(_) => {
                    let input = input.trim();
                    if input.is_empty() {
                        continue;
                    }

                    match command_handler.parse_and_execute(input) {
                        Ok(Some(output)) => {
                            println!("{}", output);

                            // Check for exit command
                            if input == "/exit" || input == "/quit" {
                                break;
                            }
                        }
                        Ok(None) => {
                            // Not a command, treat as message to send
                            let content = input.trim().to_string();
                            if !content.is_empty() {
                                match command_handler.send_message(content).await {
                                    Ok(_output) => {
                                        // Don't print "Message sent:" for cleaner chat experience
                                        // The message will appear in the message history
                                    }
                                    Err(e) => {
                                        println!("Failed to send message: {}", e);
                                    }
                                }
                            }
                        }
                        Err(CommandError::UnknownCommand(cmd)) => {
                            println!("Unknown command: {}. Type '/help' for available commands.", cmd);
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read input: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}