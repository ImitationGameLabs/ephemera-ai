//! Timer herald for Agora event hub.
//!
//! This herald produces timer events at scheduled intervals.

mod scheduler;

use clap::Parser;
use std::time::Duration;
use tracing::{error, info};
use tracing_subscriber::prelude::*;

use scheduler::TimerScheduler;

/// Timer herald configuration.
#[derive(Parser)]
#[command(name = "timer")]
struct Args {
    /// Agora server URL.
    #[arg(long, default_value = "http://localhost:8080")]
    agora_url: String,

    /// Heartbeat interval in seconds.
    #[arg(long, default_value = "30")]
    heartbeat_interval: u64,

    /// Timer tick interval in seconds.
    #[arg(long, default_value = "60")]
    tick_interval: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "timer=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();
    info!("Starting timer herald");
    info!("Agora URL: {}", args.agora_url);
    info!("Heartbeat interval: {}s", args.heartbeat_interval);
    info!("Tick interval: {}s", args.tick_interval);

    // Create scheduler
    let mut scheduler = TimerScheduler::new(
        &args.agora_url,
        "timer",
        Duration::from_secs(args.heartbeat_interval),
        Duration::from_secs(args.tick_interval),
    );

    // Run the scheduler
    if let Err(e) = scheduler.run().await {
        error!("Timer herald failed: {}", e);
        return Err(e);
    }

    Ok(())
}
