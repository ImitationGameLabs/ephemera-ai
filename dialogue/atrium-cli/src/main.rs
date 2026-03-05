mod config;

use anyhow::{anyhow, Result};
use atrium_client::{AuthenticatedClient, GetMessagesQuery};
use clap::{Parser, Subcommand};
use config::{resolve_config, MissingConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "atrium-cli")]
#[command(about = "CLI client for Dialogue Atrium chat system")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Check server connectivity
    Ping,
    /// Send a message
    Send {
        /// Message content to send
        message: String,
    },
    /// List messages
    Messages {
        /// Maximum number of messages to retrieve
        #[arg(short, long, default_value = "20")]
        limit: u64,
        /// Only retrieve messages after this ID
        #[arg(long)]
        since_id: Option<i32>,
        /// Filter by sender username
        #[arg(long)]
        sender: Option<String>,
        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,
    },
    /// List users
    Users {
        /// Show only online users
        #[arg(short, long)]
        online: bool,
        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormat,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Set a configuration value
    Set {
        /// Configuration key (server-url, auth.username, auth.password)
        key: String,
        /// Configuration value
        value: String,
    },
    /// Get a configuration value
    Get {
        /// Configuration key
        key: String,
    },
    /// List all configuration values
    List,
    /// Show configuration file path
    Path,
}

#[derive(clap::ValueEnum, Clone)]
enum OutputFormat {
    Text,
    Json,
}

fn require_config() -> Result<config::ResolvedConfig> {
    resolve_config().map_err(|missing: MissingConfig| {
        anyhow!("{}", missing.to_error_message())
    })
}

async fn create_client() -> Result<AuthenticatedClient> {
    let resolved = require_config()?;
    let bio = resolved.auth.bio.unwrap_or_default();
    let username = resolved.auth.username.clone();
    let server_url = resolved.server_url.clone();
    let client = AuthenticatedClient::connect_and_login_or_register(
        &resolved.server_url,
        resolved.auth.username,
        resolved.auth.password,
        bio,
    )
    .await
    .map_err(|e| {
        anyhow!(
            "Failed to authenticate as '{}' at {}: {}",
            username,
            server_url,
            e
        )
    })?;
    Ok(client)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Config { action } => handle_config(action)?,
        Commands::Ping => handle_ping().await?,
        Commands::Send { message } => handle_send(message).await?,
        Commands::Messages { limit, since_id, sender, format } => {
            handle_messages(limit, since_id, sender, format).await?
        }
        Commands::Users { online, format } => handle_users(online, format).await?,
    }

    Ok(())
}

fn handle_config(action: ConfigCommands) -> Result<()> {
    match action {
        ConfigCommands::Set { key, value } => {
            config::set_config_value(&key, &value)?;
            println!("Set {} = {}", key, value);
        }
        ConfigCommands::Get { key } => {
            match config::get_config_value(&key)? {
                Some(value) => println!("{}", value),
                None => println!("<not set>"),
            }
        }
        ConfigCommands::List => {
            println!("{}", config::list_config()?);
        }
        ConfigCommands::Path => {
            println!("{}", config::config_file_path());
        }
    }
    Ok(())
}

async fn handle_ping() -> Result<()> {
    let resolved = require_config()?;
    
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health", resolved.server_url))
        .send()
        .await;
    
    match response {
        Ok(resp) if resp.status().is_success() => {
            println!("pong! server-url: {}", resolved.server_url);
        }
        Ok(resp) => {
            return Err(anyhow!("Server returned status: {}", resp.status()));
        }
        Err(e) => {
            return Err(anyhow!("Failed to connect to {}: {}", resolved.server_url, e));
        }
    }
    
    Ok(())
}

async fn handle_send(message: String) -> Result<()> {
    let client = create_client().await?;

    let sent = client
        .send_message(message)
        .await
        .map_err(|e| anyhow!("Failed to send message: {}", e))?;

    println!("Message sent (id: {})", sent.id);
    Ok(())
}

async fn handle_messages(
    limit: u64,
    since_id: Option<i32>,
    sender: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    let client = create_client().await?;

    let query = GetMessagesQuery {
        sender,
        limit: Some(limit),
        offset: None,
        since_id,
    };

    let response = client
        .get_messages(query)
        .await
        .map_err(|e| anyhow!("Failed to get messages: {}", e))?;

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({ "messages": response.messages }))?
            );
        }
        OutputFormat::Text => {
            if response.messages.is_empty() {
                println!("No messages found.");
            } else {
                for msg in &response.messages {
                    println!("[{}] {}: {}", msg.created_at, msg.sender, msg.content);
                }
            }
        }
    }

    Ok(())
}

async fn handle_users(online: bool, format: OutputFormat) -> Result<()> {
    let client = create_client().await?;

    let response = client
        .get_all_users()
        .await
        .map_err(|e| anyhow!("Failed to get users: {}", e))?;

    let users = if online {
        response
            .users
            .into_iter()
            .filter(|u| u.status.online)
            .collect::<Vec<_>>()
    } else {
        response.users
    };

    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({ "users": users }))?
            );
        }
        OutputFormat::Text => {
            if users.is_empty() {
                println!(
                    "No {}users found.",
                    if online { "online " } else { "" }
                );
            } else {
                for user in &users {
                    let status = if user.status.online {
                        "online".to_string()
                    } else if let Some(last_seen) = &user.status.last_seen {
                        format!("offline (last seen: {})", last_seen)
                    } else {
                        "offline".to_string()
                    };
                    println!("{} - {} [{}]", user.name, user.bio, status);
                }
            }
        }
    }

    Ok(())
}
