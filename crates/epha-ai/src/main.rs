use crate::agent::EphemeraAI;
use clap::Parser;
use llm::builder::{LLMBackend, LLMBuilder};
use llm::chat::ChatMessage;
use loom_client::LoomClient;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;
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
        .expect("Failed to init loom client");

    // Validate LLM credentials before starting
    validate_llm_config(&config.llm)
        .await
        .expect("LLM validation failed");

    let loom_client = Arc::new(loom_client);

    let mut ai = EphemeraAI::new(config, loom_client.clone(), http_client).await?;
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

/// Validate LLM credentials by sending a test message.
async fn validate_llm_config(llm_config: &crate::config::LlmConfig) -> anyhow::Result<()> {
    info!("Validating LLM credentials at: {}", llm_config.base_url);

    let llm = LLMBuilder::new()
        .backend(LLMBackend::Groq)
        .api_key(&llm_config.api_key)
        .base_url(&llm_config.base_url)
        .model(&llm_config.model)
        .system("Reply ok.")
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build LLM client: {}", e))?;

    // Send a test message
    let response = llm
        .chat(&[ChatMessage::user().content("hi").build()])
        .await
        .map_err(|e| anyhow::anyhow!("LLM API validation failed: {}", e))?;

    info!(
        "LLM credentials validated successfully. Response: {}",
        response.text().unwrap_or_default()
    );

    Ok(())
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
