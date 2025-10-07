use clap::Parser;
use dotenv::dotenv;
use std::env;
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt
};

// Import the CLI modules from the parent directory
use dialogue_atrium::cli::CliInterface;

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

    // Create CLI client
    let client = dialogue_atrium::cli::DialogueClient::new(server_url);
    let mut interface = CliInterface::new(client);

    // Run the CLI interface with optional pre-auth credentials
    if let Err(e) = interface.run_with_credentials(args.user, args.password).await {
        eprintln!("CLI error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}