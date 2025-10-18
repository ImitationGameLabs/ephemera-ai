use dotenv::dotenv;
use epha_memory::HybridMemoryManager;
use rig::providers::{deepseek, openai};
use rig::client::embeddings::EmbeddingsClientDyn;
use qdrant_client::config::QdrantConfig;
use sea_orm_migration::MigratorTrait;
use tracing::info;
use std::sync::Arc;
use crate::agent::EphemeraAI;

mod agent;
mod tools;  

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let model_name = std::env::var("MODEL_NAME")
        .expect("MODEL_NAME not set");

    let llm_client = init_llm_client();

    let memory_manager = init_memory_manager()
        .await
        .expect("Failed to init memory manager");

    let mut ai = EphemeraAI::new(llm_client, Arc::new(memory_manager), &model_name);
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

async fn init_memory_manager() -> anyhow::Result<HybridMemoryManager> {
    // Setup MySQL connection
    let mysql_url = std::env::var("EPHA_MEMORY_MYSQL_URL").expect("EPHA_MEMORY_MYSQL_URL not set");
    let conn = sea_orm::Database::connect(&mysql_url).await?;

    // Run database migrations
    info!("Running database migrations...");
    epha_memory::Migrator::up(&conn, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
    info!("Migrations completed successfully!");

    // Setup Qdrant connection
    let qdrant_url = std::env::var("EPHA_MEMORY_QDRANT_URL").expect("EPHA_MEMORY_QDRANT_URL not set");
    let qdrant_config = QdrantConfig {
        uri: qdrant_url.clone(),
        ..Default::default()
    };
    let qdrant_client =
        qdrant_client::Qdrant::new(qdrant_config).expect("Failed to create Qdrant client");

    // Initialize embedding model
    let embedding_model_name = std::env::var("EMBEDDING_MODEL")
        .expect("EMBEDDING_MODEL not set");
    let embedding_api_key = std::env::var("EMBEDDING_MODEL_API_KEY")
        .expect("EMBEDDING_MODEL_API_KEY not set");
    let embedding_url = std::env::var("EMBEDDING_MODEL_URL")
        .expect("EMBEDDING_MODEL_URL not set");

    
    // Create OpenAI-compatible client for custom embedding service
    let embedding_client = openai::Client::builder(&embedding_api_key)
        .base_url(&embedding_url)
        .build();

    // Get embedding dimensions (required)
    let embedding_dimensions: usize = std::env::var("EMBEDDING_MODEL_DIMENSIONS")
        .expect("EMBEDDING_MODEL_DIMENSIONS not set")
        .parse()
        .expect("EMBEDDING_MODEL_DIMENSIONS must be a valid number");

    let embedding_model = embedding_client.embedding_model_with_ndims(&embedding_model_name, embedding_dimensions);

    let memory_manager = HybridMemoryManager::new(
        epha_memory::MysqlMemoryManager::new(conn),
        epha_memory::QdrantMemoryManager::new(qdrant_client, embedding_dimensions),
        embedding_model,
    );

    Ok(memory_manager)
}