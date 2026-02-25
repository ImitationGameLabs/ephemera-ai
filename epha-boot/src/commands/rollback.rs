use crate::{config::Config, shell};
use anyhow::Result;
use std::process::Command;

pub fn execute(config: &Config) -> Result<()> {
    shell::ensure_flake_exists()?;

    println!("Rolling back profile {}...", config.profile_name);

    shell::run(
        Command::new("nix")
            .args(["profile", "rollback", "--profile-name", &config.profile_name]),
    )?;

    println!("Rollback complete!");
    Ok(())
}
