use dotenv::dotenv;
use loom_client::LoomClient;
use atrium_client::AuthenticatedClient;
use rig::providers::deepseek;
use tracing::info;
use std::sync::Arc;
use crate::agent::EphemeraAI;

mod agent;
mod tools;
mod context;  

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let model_name = std::env::var("MODEL_NAME")
        .expect("MODEL_NAME not set");

    let llm_client = init_llm_client();

    let loom_client = init_loom_client()
        .await
        .expect("Failed to init loom client");

    let dialogue_client = init_dialogue_client()
        .await
        .expect("Failed to init dialogue client");

    let loom_client = Arc::new(loom_client);
    let dialogue_client = Arc::new(dialogue_client);
    let mut ai = EphemeraAI::new(llm_client, loom_client, dialogue_client, &model_name);
    ai.run().await?;

    Ok(())
}

fn init_llm_client() -> deepseek::Client {
    // Create LLM client (OpenAI-compatible)
    let api_key = std::env::var("API_KEY")
        .expect("API_KEY not set");
    let base_url = std::env::var("BASE_URL")
        .expect("BASE_URL not set");

    let llm_client = deepseek::Client::builder(&api_key)
        .base_url(&base_url)
        .build();

    llm_client
}

async fn init_loom_client() -> anyhow::Result<LoomClient> {
    // Setup loom service connection
    let loom_service_url = std::env::var("LOOM_SERVICE_URL")
        .expect("LOOM_SERVICE_URL environment variable not set");

    info!("Connecting to loom service at: {}", loom_service_url);

    // Test connection with health check
    let client = LoomClient::new(&loom_service_url);
    client.health_check()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to loom service: {}", e))?;

    info!("Successfully connected to loom service!");

    Ok(client)
}

async fn init_dialogue_client() -> anyhow::Result<AuthenticatedClient> {
    // Setup atrium service connection
    let atrium_service_url = std::env::var("ATRIUM_SERVICE_URL")
        .expect("ATRIUM_SERVICE_URL environment variable not set");

    info!("Connecting to atrium service at: {}", atrium_service_url);

    // Read credentials from environment variables (application layer responsibility)
    let username = std::env::var("ATRIUM_USERNAME")
        .map_err(|_| anyhow::anyhow!("ATRIUM_USERNAME environment variable not set"))?;

    let password = std::env::var("ATRIUM_PASSWORD")
        .map_err(|_| anyhow::anyhow!("ATRIUM_PASSWORD environment variable not set"))?;

    info!("Logging in...");

    let authenticated_client = AuthenticatedClient::connect_and_login(&atrium_service_url, username, password).await
        .map_err(|e| anyhow::anyhow!("Failed to login: {}", e))?;

    info!("Successfully logged in as: {}!",
          authenticated_client.user().await
          .ok_or_else(|| anyhow::anyhow!("User info not available"))?
          .name);

    Ok(authenticated_client)
}