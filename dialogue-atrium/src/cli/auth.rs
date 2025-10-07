use std::io::{self, Write};
use rpassword::read_password;
use crate::cli::client::{DialogueClient, ClientError};
use crate::models::{CreateUserRequest, UserResponse};

#[derive(Debug, Clone)]
pub struct AuthSession {
    pub username: String,
    pub password: String,
    pub user_info: UserResponse,
}

pub struct AuthManager {
    client: DialogueClient,
}

impl AuthManager {
    pub fn new(client: DialogueClient) -> Self {
        Self { client }
    }

    fn prompt_username() -> String {
        print!("Enter username: ");
        io::stdout().flush().unwrap();

        let mut username = String::new();
        io::stdin().read_line(&mut username).expect("Failed to read username");
        username.trim().to_string()
    }

    fn prompt_password() -> String {
        print!("Enter password: ");
        io::stdout().flush().unwrap();
        read_password().expect("Failed to read password")
    }

    fn prompt_bio() -> String {
        print!("Enter bio (optional): ");
        io::stdout().flush().unwrap();

        let mut bio = String::new();
        io::stdin().read_line(&mut bio).expect("Failed to read bio");
        bio.trim().to_string()
    }

    fn prompt_register() -> bool {
        println!("User not found or invalid credentials.");
        print!("Would you like to register? (y/n): ");
        io::stdout().flush().unwrap();

        let mut response = String::new();
        io::stdin().read_line(&mut response).expect("Failed to read response");

        matches!(response.trim().to_lowercase().as_str(), "y" | "yes")
    }

    pub async fn authenticate_or_register(&self) -> Result<AuthSession, Box<dyn std::error::Error>> {
        println!("=== Dialogue Atrium CLI ===");

        loop {
            let username = Self::prompt_username();
            let password = Self::prompt_password();

            // Try to authenticate first
            match self.client.authenticate(&username, &password).await {
                Ok(user_info) => {
                    println!("✓ Successfully logged in as: {}", user_info.name);
                    return Ok(AuthSession {
                        username,
                        password,
                        user_info,
                    });
                }
                Err(ClientError::ApiError(msg)) if msg.contains("Invalid password") => {
                    println!("✗ Invalid password. Please try again.\n");
                }
                Err(ClientError::ApiError(msg)) if msg.contains("404") || msg.contains("not found") => {
                    // User doesn't exist
                    if Self::prompt_register() {
                        let bio = Self::prompt_bio();

                        let register_request = CreateUserRequest {
                            name: username.clone(),
                            bio: if bio.is_empty() { "No bio provided".to_string() } else { bio },
                            password: password.clone(),
                        };

                        match self.client.register_user(register_request).await {
                            Ok(user_info) => {
                                println!("✓ Successfully registered and logged in as: {}", user_info.name);
                                return Ok(AuthSession {
                                    username,
                                    password,
                                    user_info,
                                });
                            }
                            Err(e) => {
                                println!("✗ Registration failed: {}", e);
                                println!("Please try again.\n");
                            }
                        }
                    } else {
                        println!("Please try again.\n");
                    }
                }
                Err(e) => {
                    println!("✗ Authentication failed: {}", e);
                    println!("Please try again.\n");
                }
            }
        }
    }

    pub async fn authenticate_or_register_with_credentials(&self, username: String, password: String) -> Result<AuthSession, Box<dyn std::error::Error>> {
        println!("=== Dialogue Atrium CLI ===");

        loop {
            // Try to authenticate first
            match self.client.authenticate(&username, &password).await {
                Ok(user_info) => {
                    println!("✓ Successfully logged in as: {}", user_info.name);
                    return Ok(AuthSession {
                        username,
                        password,
                        user_info,
                    });
                }
                Err(ClientError::ApiError(msg)) if msg.contains("Invalid password") => {
                    println!("✗ Invalid password. Please try again.");
                    // Fall back to interactive mode
                    return self.authenticate_or_register().await;
                }
                Err(ClientError::ApiError(msg)) if msg.contains("404") || msg.contains("not found") => {
                    // User doesn't exist
                    println!("✗ User '{}' not found.", username);
                    // Fall back to interactive mode
                    return self.authenticate_or_register().await;
                }
                Err(e) => {
                    println!("✗ Authentication failed: {}", e);
                    // Fall back to interactive mode
                    return self.authenticate_or_register().await;
                }
            }
        }
    }
}