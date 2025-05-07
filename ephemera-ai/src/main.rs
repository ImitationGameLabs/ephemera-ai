use ephemera_memory::MeiliMemoryManager;
use rig::providers::deepseek;
use tracing::info;
use dotenv::dotenv;

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

    let meili_url = std::env::var("MEILISEARCH_URL")
        .expect("MEILISEARCH_URL not set");
    let meili_api_key = std::env::var("MEILI_MASTER_KEY")
        .expect("MEILI_MASTER_KEY not set");
    let meili_client = meilisearch_sdk::client::Client::new(meili_url, Some(meili_api_key)).unwrap();

    let stats = meili_client.get_stats().await?;
    info!("Hello, meilisearch stats info: {:?}", stats);

    let memory_manager = MeiliMemoryManager::new(meili_client);

    let agent = ephemera::Ephemera {
        chat_agent,
        keyword_agent,

        chat_history: Vec::new(),
        memory_manager,
    };

    let mut ui= interface::Cli{
        agent,
    };

    ui.run().await
}