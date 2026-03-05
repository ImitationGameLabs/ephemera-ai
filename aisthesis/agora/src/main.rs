mod config;
mod event;
mod handlers;
mod herald;
mod queue;
mod server;

use clap::Parser;
use std::path::PathBuf;
use tracing::info;

use crate::config::Config;
use crate::server::AgoraServer;

#[derive(Parser)]
#[command(name = "agora")]
struct Args {
    /// Directory containing config files.
    #[arg(long, default_value = ".config")]
    config_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config_path = args.config_dir.join("agora.json");
    let config = Config::load(&config_path);

    info!("Starting Agora Event Hub on {}", config.bind_address());

    let server = AgoraServer::new(config).await?;
    server.run().await?;

    Ok(())
}
