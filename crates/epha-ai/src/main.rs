use crate::agent::EphemeraAI;
use clap::Parser;
use loom_client::LoomClient;
use rig::providers::deepseek;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

mod agent;
mod config;
mod context;
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

    tracing_subscriber::fmt::init();

    let llm_client = init_llm_client(&config.llm);

    let loom_client = init_loom_client(&config.services.loom_url)
        .await
        .expect("Failed to init loom client");

    let llm_client = init_llm_client(&config.llm);

    let loom_client = Arc::new(loom_client);

    let mut ai = EphemeraAI::new(config, loom_client, llm_client).await?;
    ai.live().await?;

    Ok(())
}

fn init_llm_client(llm_config: &crate::config::LlmConfig) -> deepseek::Client {
    deepseek::Client::builder(&llm_config.api_key)
        .base_url(&llm_config.base_url)
        .build()
}

async fn init_loom_client(loom_url: &str) -> anyhow::Result<LoomClient> {
    info!("Connecting to loom service at: {}", loom_url);

    // Test connection with health check
    let client = LoomClient::new(loom_url);
    client
        .health_check()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to loom service: {}", e))?;

    info!("Successfully connected to loom service!");

    Ok(client)
}
