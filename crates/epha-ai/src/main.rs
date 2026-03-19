use crate::agent::EphemeraAI;
use clap::Parser;
use loom_client::LoomClient;
use reqwest::Client;
use rig::client::CompletionClient;
use rig::completion::Completion;
use rig::providers::deepseek;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

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

    tracing_subscriber::fmt::init();

    let http_client = build_http_client();
    let loom_client = init_loom_client(&config.services.loom_url, http_client.clone())
        .await
        .expect("Failed to init loom client");

    let llm_client = init_llm_client(&config.llm).await.expect("Failed to init LLM client");
    let loom_client = Arc::new(loom_client);

    let mut ai = EphemeraAI::new(config, loom_client.clone(), llm_client, http_client).await?;
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

async fn init_llm_client(
    llm_config: &crate::config::LlmConfig,
) -> anyhow::Result<deepseek::Client> {
    info!("Validating LLM credentials at: {}", llm_config.base_url);

    let client =
        deepseek::Client::builder(&llm_config.api_key).base_url(&llm_config.base_url).build();

    // Create a minimal agent to test the connection
    let agent = client.agent(&llm_config.model).preamble("Reply ok.").build();

    // Send a test message
    agent
        .completion("hi", vec![])
        .await?
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("LLM API validation failed: {}", e))?;

    info!("LLM credentials validated successfully");

    Ok(client)
}

async fn init_loom_client(loom_url: &str, http_client: Client) -> anyhow::Result<LoomClient> {
    info!("Connecting to loom service at: {}", loom_url);

    // Test connection with health check
    let client = LoomClient::new(loom_url, http_client);
    client
        .health_check()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to loom service: {}", e))?;

    info!("Successfully connected to loom service!");

    Ok(client)
}
