use dotenv::dotenv;
use ephemera_memory::HybridMemoryManager;
use fastembed::{EmbeddingModel, TextEmbedding, InitOptions};
use qdrant_client::config::QdrantConfig;
use rig::providers::deepseek;

mod ephemera;
mod interface;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    // Create Deepseek client
    let deepseek_client = deepseek::Client::from_env();

    let chat_agent = deepseek_client
        .agent(deepseek::DEEPSEEK_CHAT)
        .preamble("You are a Super AI with persistent memory capabilities.")
        .tool(ephemera::Add)
        .build();

    let keyword_agent = deepseek_client
        .agent(deepseek::DEEPSEEK_CHAT)
        .preamble("Extract keywords from the context. Return keywords only, separated by spaces.")
        .build();

    // Setup MySQL connection
    let mysql_url = std::env::var("MYSQL_URL").expect("MYSQL_URL not set");
    let conn = sea_orm::Database::connect(&mysql_url).await?;

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
        .unwrap_or_else(|_| "BGESmallENV15".to_string());
    let embedding_model_enum: EmbeddingModel = model_name.parse()
        .map_err(|_| anyhow::anyhow!("Invalid embedding model name: {}", model_name))?;

    let mut init_options = InitOptions::default();
    init_options.model_name = embedding_model_enum;
    init_options.show_download_progress = true;

    let embedding_model = TextEmbedding::try_new(init_options)
    .map_err(|e| anyhow::anyhow!("Failed to initialize embedding model {}: {}", model_name, e))?;

    let memory_manager = HybridMemoryManager::new(
        ephemera_memory::MysqlMemoryManager::new(conn),
        ephemera_memory::QdrantMemoryManager::new(qdrant_client),
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
