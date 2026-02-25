use crate::{config::Config, shell};
use anyhow::Result;
use std::process::Command;

pub fn execute(config: &Config) -> Result<()> {
    shell::ensure_flake_exists()?;

    println!("History for profile {}:", config.profile_name);
    println!("============================\n");

    shell::run(
        Command::new("nix")
            .args(["profile", "history", "--profile-name", &config.profile_name]),
    )?;

    Ok(())
}
