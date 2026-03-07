mod cli;
mod commands;
mod config;
mod shell;

use clap::Parser;
use config::Config;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    let config = Config::default();

    let result = match cli.command {
        cli::Commands::Init { template } => {
            let template_url = template.unwrap_or_else(|| {
                format!("{}#{}", config.project_url, config.template)
            });
            commands::init(&template_url)
        }
        cli::Commands::Update { input } => commands::update(input.as_deref()),
        cli::Commands::Build => commands::build(&config),
        cli::Commands::Switch => {
            commands::build(&config)?;
            commands::switch(&config)
        }
        cli::Commands::Rollback => commands::rollback(&config),
        cli::Commands::Status => commands::status(&config),
        cli::Commands::History => commands::history(&config),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
