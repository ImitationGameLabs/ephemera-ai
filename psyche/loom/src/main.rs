mod services;
mod server;
mod memory;

use clap::{Arg, Command};
use dotenv::dotenv;
use tracing::info;

use crate::server::{LoomServer, ServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv().ok();

    let matches = Command::new("loom-server")
        .version("0.1.0")
        .about("Loom Memory Service HTTP Server")
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .value_name("HOST")
                .help("Sets the host address to bind to")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the port to bind to")
                .default_value("8080"),
        )
        .get_matches();

    let config = ServerConfig {
        host: matches.get_one::<String>("host").unwrap().clone(),
        port: matches.get_one::<String>("port")
            .unwrap()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid port number"))?,
    };

    info!("Starting Loom Memory Service on {}:{}", config.host, config.port);

    let server = LoomServer::new(config).await?;
    server.run().await?;

    Ok(())
}