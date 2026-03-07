mod config;
mod memory;
mod server;
mod services;

use clap::Parser;
use std::path::PathBuf;
use tracing::info;

use crate::config::Config;
use crate::server::LoomServer;

#[derive(Parser)]
#[command(name = "loom")]
struct Args {
    /// Directory containing config files
    #[arg(long, default_value = ".config")]
    config_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config_path = args.config_dir.join("loom.json");
    let config = Config::load(&config_path);

    info!("Starting Loom Memory Service on {}", config.bind_address());

    let server = LoomServer::new(config).await?;
    server.run().await?;

    Ok(())
}
