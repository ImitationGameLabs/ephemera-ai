use rig::completion::CompletionModel;
use rig::embeddings::embedding::EmbeddingModel;
use tracing::info;

use std::io;

use crate::ephemera::Ephemera;

pub struct Cli<M: CompletionModel, T: EmbeddingModel + Send + Sync> {
    pub agent: Ephemera<M, T>,
}

impl<M: CompletionModel, T: EmbeddingModel + Send + Sync> Cli<M, T> {
    pub async fn run(&mut self) -> anyhow::Result<()> {
        info!("Start running");

        loop {
            println!("Please input you prompt: ");

            let mut buf = String::new();

            io::stdin()
                .read_line(&mut buf)
                .expect("Failed to read from stdin");

            let res = self.agent.prompt(buf).await?;

            println!("Response: {}", res)
        }
    }
}
