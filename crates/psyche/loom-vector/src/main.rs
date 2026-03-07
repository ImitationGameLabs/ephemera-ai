mod memory;
mod server;
mod services;

use dotenv::dotenv;
use std::env;
use tracing::info;

use crate::server::{LoomVectorServer, ServerConfig};

/// Get service port from environment variable with default
fn get_service_port() -> u16 {
    env::var("LOOM_VECTOR_SERVICE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3003)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv().ok();

    // Get service port from environment variable
    let port = get_service_port();
    let config = ServerConfig { port };

    info!(
        "Starting Loom Vector Search Service on {}",
        config.bind_address()
    );

    let server = LoomVectorServer::new(config).await?;
    server.run().await?;

    Ok(())
}
