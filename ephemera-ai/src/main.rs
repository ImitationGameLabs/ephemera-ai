use rig::providers::deepseek;

mod ephemera;
mod interface;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // tracing_subscriber::registry()
    //     .with(
    //         tracing_subscriber::EnvFilter::try_from_default_env()
    //             .unwrap_or_else(|_| "stdout=info".into()),
    //     )
    //     .with(tracing_subscriber::fmt::layer())
    //     .init();

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