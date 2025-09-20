use rig::completion::CompletionModel;
use tracing::info;

use std::io;

use crate::ephemera::Ephemera;

pub struct Cli<M: CompletionModel> {
    pub agent: Ephemera<M>,
}

impl<M: CompletionModel> Cli<M> {
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
