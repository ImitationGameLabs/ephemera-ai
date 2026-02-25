use crate::{config::Config, shell};
use anyhow::Result;
use std::process::Command;

pub fn execute(config: &Config) -> Result<()> {
    shell::ensure_flake_exists()?;

    println!("Profile: {}", config.profile_name);
    println!("==================\n");

    shell::run(
        Command::new("nix")
            .args(["profile", "list", "--profile-name", &config.profile_name]),
    )?;

    Ok(())
}
