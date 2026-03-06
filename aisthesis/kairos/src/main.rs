mod config;
mod schedule;
mod scheduler;
mod server;
mod store;

use clap::Parser;
use std::path::PathBuf;
use tracing::info;

use crate::config::Config;
use crate::server::KairosServer;

#[derive(Parser)]
#[command(name = "kairos")]
struct Args {
    /// Directory containing config files.
    #[arg(long, default_value = ".config")]
    config_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config_path = args.config_dir.join("kairos.json");
    let config = Config::load(&config_path);

    info!("Starting Kairos Time Management Service on {}", config.bind_address());

    let server = KairosServer::new(config).await?;
    server.run().await?;

    Ok(())
}
