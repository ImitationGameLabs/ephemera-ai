use dotenv::dotenv;
use ephemera_memory::HybridMemoryManager;
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

    let memory_manager = HybridMemoryManager::new(
        ephemera_memory::MysqlMemoryManager::new(conn),
        ephemera_memory::QdrantMemoryManager::new(qdrant_client),
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
