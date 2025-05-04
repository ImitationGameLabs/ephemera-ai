use rig::providers::deepseek;

mod ephemera;
mod interface;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Create Deepseek client
    let deepseek_client = deepseek::Client::from_env();

    // Create RAG agent with a single context prompt and a dynamic tool source
    let rag = deepseek_client
        .agent(deepseek::DEEPSEEK_CHAT)
        .preamble("You are Super AI with infinte memory.")
        .tool(ephemera::Add)
        .build();

    let agent = ephemera::Ephemera {
        agent: rag,
        chat_history: Vec::new(),
    };

    let mut ui= interface::Cli{
        agent,
    };

    ui.run().await
}