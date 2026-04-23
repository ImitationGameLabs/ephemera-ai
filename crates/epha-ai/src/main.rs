use crate::agent::EphemeraAI;
use clap::Parser;
use loom_client::LoomClient;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod agent;
mod config;
mod context;
mod sync;
mod tools;

use crate::config::Config;

#[derive(Parser)]
#[command(name = "epha-ai")]
struct Args {
    /// Directory containing config files
    #[arg(long, default_value = ".config")]
    config_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let config_path = args.config_dir.join("epha-ai.json");
    let config = Config::load(&config_path);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let http_client = build_http_client();
    let loom_client = init_loom_client(&config.services.loom_url, http_client.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Startup step failed (loom connectivity): {:#}", e))?;

    let loom_client = Arc::new(loom_client);

    let mut ai = EphemeraAI::new(config, loom_client.clone(), http_client)
        .await
        .map_err(|e| anyhow::anyhow!("Startup step failed (EphemeraAI initialization): {:#}", e))?;

    ai.validate_llm_access()
        .await
        .map_err(|e| anyhow::anyhow!("Startup step failed (LLM credential validation): {:#}", e))?;

    ai.live().await?;

    Ok(())
}

fn build_http_client() -> Client {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

async fn init_loom_client(loom_url: &str, http_client: Client) -> anyhow::Result<LoomClient> {
    info!("Connecting to loom service at: {}", loom_url);

    let client = LoomClient::new(loom_url, http_client);
    let mut attempt = 0u32;
    let mut delay = Duration::from_secs(1);
    loop {
        match client.health_check().await {
            Ok(_) => {
                info!("Successfully connected to loom service!");
                return Ok(client);
            }
            Err(e) => {
                attempt += 1;
                warn!(
                    "Loom health check failed (attempt {attempt}): {e}. Retrying in {}s...",
                    delay.as_secs()
                );
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(Duration::from_secs(30));
            }
        }
    }
}
