use std::io::{self, Write};
use rpassword::read_password;
use atrium_client::AuthenticatedClient;

pub struct CliAuthPrompt;

impl CliAuthPrompt {
    pub fn prompt_username() -> String {
        print!("Enter username: ");
        io::stdout().flush().unwrap();

        let mut username = String::new();
        io::stdin().read_line(&mut username).expect("Failed to read username");
        username.trim().to_string()
    }

    pub fn prompt_password() -> String {
        print!("Enter password: ");
        io::stdout().flush().unwrap();
        read_password().expect("Failed to read password")
    }

    pub fn prompt_bio() -> String {
        print!("Enter bio (optional): ");
        io::stdout().flush().unwrap();

        let mut bio = String::new();
        io::stdin().read_line(&mut bio).expect("Failed to read bio");
        bio.trim().to_string()
    }

    pub fn prompt_register() -> bool {
        println!("User not found or invalid credentials.");
        print!("Would you like to register? (y/n): ");
        io::stdout().flush().unwrap();

        let mut response = String::new();
        io::stdin().read_line(&mut response).expect("Failed to read response");

        matches!(response.trim().to_lowercase().as_str(), "y" | "yes")
    }

    pub async fn authenticate_or_register_interactive(server_url: &str) -> Result<AuthenticatedClient, Box<dyn std::error::Error>> {
        println!("=== Dialogue Atrium CLI ===");

        loop {
            let username = Self::prompt_username();
            let password = Self::prompt_password();

            // Try to login first
            match AuthenticatedClient::connect_and_login(server_url, username.clone(), password.clone()).await {
                Ok(client) => {
                    println!("✓ Successfully logged in as: {}",
                        client.user().await
                            .ok_or("User info not available")?
                            .name);
                    return Ok(client);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("Invalid password") {
                        println!("✗ Invalid password. Please try again.\n");
                    } else if error_msg.contains("404") || error_msg.contains("not found") {
                        // User doesn't exist - offer registration
                        if Self::prompt_register() {
                            let bio = Self::prompt_bio();

                            match AuthenticatedClient::connect_and_login_or_register(
                                server_url,
                                username,
                                password,
                                if bio.is_empty() { "".to_string() } else { bio }
                            ).await {
                                Ok(client) => {
                                    println!("✓ Successfully registered and logged in as: {}",
                                        client.user().await
                                            .ok_or("User info not available")?
                                            .name);
                                    return Ok(client);
                                }
                                Err(e) => {
                                    println!("✗ Registration failed: {}", e);
                                    println!("Please try again.\n");
                                }
                            }
                        } else {
                            println!("Please try again.\n");
                        }
                    } else {
                        println!("✗ Authentication failed: {}", e);
                        println!("Please try again.\n");
                    }
                }
            }
        }
    }
}