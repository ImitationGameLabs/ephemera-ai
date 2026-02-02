mod services;
mod server;
mod memory;
mod system_configs;

use dotenv::dotenv;
use tracing::info;
use std::env;

use crate::server::{LoomServer, ServerConfig};

/// Get service port from environment variable with default
fn get_service_port() -> u16 {
    env::var("LOOM_SERVICE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3001)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv().ok();

    // Get service port from environment variable
    let port = get_service_port();
    let config = ServerConfig { port };

    info!("Starting Loom Memory Service on {}",
          config.bind_address());

    let server = LoomServer::new(config).await?;
    server.run().await?;

    Ok(())
}