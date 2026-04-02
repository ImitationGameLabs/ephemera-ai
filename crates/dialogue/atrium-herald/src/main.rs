//! Atrium herald for Agora event hub.
//!
//! This herald produces chat.message events from atrium.

mod config;
mod poller;

use clap::Parser;
use reqwest::Client;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info};
use tracing_subscriber::prelude::*;

use crate::config::Config;
use crate::poller::Poller;

/// Atrium herald configuration.
#[derive(Parser)]
#[command(name = "atrium-herald")]
#[command(about = "Atrium herald for Agora event hub")]
struct Args {
    /// Config directory path
    #[arg(long)]
    config_dir: PathBuf,
}

fn build_http_client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "atrium_herald=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();
    let config_path = args.config_dir.join("config.json");
    let config = Config::load(&config_path);

    info!("Starting atrium herald");
    info!("Atrium URL: {}", config.atrium_url);
    info!("Agora URL: {}", config.agora_url);
    info!("Username: {}", config.username);
    info!("Poll interval: {}ms", config.poll_interval_ms);
    info!(
        "Agora heartbeat interval: {}ms",
        config.heartbeat_interval_ms
    );
    info!(
        "Atrium heartbeat interval: {}ms",
        config.atrium_heartbeat_interval_ms
    );

    let http_client = build_http_client();

    // Login or register to atrium
    info!("Connecting to atrium...");
    let atrium_client = atrium_client::AuthenticatedClient::connect_and_login_or_register(
        &config.atrium_url,
        config.username.clone(),
        config.password.clone(),
        config.bio.clone().unwrap_or_default(),
        http_client.clone(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to connect to atrium: {}", e))?;
    info!("Successfully connected to atrium");

    // Create poller
    let mut poller = Poller::new(
        atrium_client,
        "atrium-herald",
        &config.agora_url,
        Duration::from_millis(config.poll_interval_ms),
        Duration::from_millis(config.heartbeat_interval_ms),
        Duration::from_millis(config.atrium_heartbeat_interval_ms),
        http_client,
    );

    // Run the poller
    if let Err(e) = poller.run().await {
        error!("Atrium herald failed: {}", e);
        return Err(e);
    }

    Ok(())
}
