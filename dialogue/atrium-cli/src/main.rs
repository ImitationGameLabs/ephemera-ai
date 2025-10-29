mod commands;
mod interface;
mod auth_cli;

use clap::Parser;
use dotenv::dotenv;
use std::env;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt
};

use crate::interface::CliInterface;

#[derive(Parser)]
#[command(name = "dialogue-cli")]
#[command(about = "CLI client for Dialogue Atrium chat system")]
#[command(version)]
struct Args {
    /// Server URL to connect to
    #[arg(short, long, default_value = "http://127.0.0.1:3000")]
    server: String,

    /// Username for quick login (optional)
    #[arg(short, long)]
    user: Option<String>,

    /// Password for quick login (optional)
    #[arg(short, long)]
    password: Option<String>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let args = Args::parse();

    // Initialize logging
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Use server URL from args or environment variable
    let server_url = env::var("DIALOGUE_ATRIUM_SERVER_URL")
        .unwrap_or_else(|_| args.server);

    tracing::info!("Connecting to Dialogue Atrium server at: {}", server_url);

    // Create CLI interface
    let mut interface = CliInterface::new(server_url);

    // Run the CLI interface with optional pre-auth credentials
    if let Err(e) = interface.run(args.user, args.password).await {
        eprintln!("CLI error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}