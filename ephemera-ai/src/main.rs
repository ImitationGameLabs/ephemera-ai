use dotenv::dotenv;
use ephemera_memory::HybridMemoryManager;
use rig::providers::openai;
use qdrant_client::config::QdrantConfig;
use sea_orm_migration::MigratorTrait;

mod ephemera;
mod interface;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    // Create LLM client (OpenAI-compatible)
    let api_key = std::env::var("API_KEY")
        .expect("API_KEY not set");
    let base_url = std::env::var("BASE_URL")
        .expect("BASE_URL not set");
    let model_name = std::env::var("MODEL_NAME")
        .expect("MODEL_NAME not set");

    let llm_client = openai::Client::from_url(&api_key, &base_url);

    let chat_agent = llm_client
        .agent(&model_name)
        .preamble("You are a Super AI with persistent memory capabilities.")
        .tool(ephemera::Add)
        .build();

    let keyword_agent = llm_client
        .agent(&model_name)
        .preamble("Extract keywords from the context. Return keywords only, separated by spaces.")
        .build();

    // Setup MySQL connection
    let mysql_url = std::env::var("MYSQL_URL").expect("MYSQL_URL not set");
    let conn = sea_orm::Database::connect(&mysql_url).await?;

    // Run database migrations
    println!("Running database migrations...");
    ephemera_memory::Migrator::up(&conn, None)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
    println!("Migrations completed successfully!");

    // Setup Qdrant connection
    let qdrant_url = std::env::var("QDRANT_URL").expect("QDRANT_URL not set");
    let qdrant_config = QdrantConfig {
        uri: qdrant_url.clone(),
        ..Default::default()
    };
    let qdrant_client =
        qdrant_client::Qdrant::new(qdrant_config).expect("Failed to create Qdrant client");

    // Initialize embedding model
    let model_name = std::env::var("EMBEDDING_MODEL")
        .expect("EMBEDDING_MODEL not set");
    let embedding_api_key = std::env::var("EMBEDDING_MODEL_API_KEY")
        .expect("EMBEDDING_MODEL_API_KEY not set");
    let embedding_url = std::env::var("EMBEDDING_MODEL_URL")
        .expect("EMBEDDING_MODEL_URL not set");

    // Create OpenAI-compatible client for custom embedding service
    let openai_client = openai::Client::from_url(&embedding_api_key, &embedding_url);

    // Get embedding dimensions (required)
    let embedding_dimensions: usize = std::env::var("EMBEDDING_MODEL_DIMENSIONS")
        .expect("EMBEDDING_MODEL_DIMENSIONS not set")
        .parse()
        .expect("EMBEDDING_MODEL_DIMENSIONS must be a valid number");

    let embedding_model = openai_client.embedding_model_with_ndims(&model_name, embedding_dimensions);

    let memory_manager = HybridMemoryManager::new(
        ephemera_memory::MysqlMemoryManager::new(conn),
        ephemera_memory::QdrantMemoryManager::new(qdrant_client, embedding_dimensions),
        embedding_model,
    );

    let agent = ephemera::Ephemera {
        chat_agent,
        keyword_agent,

        chat_history: Vec::new(),
        memory_manager,
    };

    let mut ui = interface::Cli { agent };

    ui.run().await
}
