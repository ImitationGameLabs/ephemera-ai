use crate::config::Config;
use anyhow::{bail, Result};
use std::process::Command;

pub fn execute(config: &Config) -> Result<()> {
    println!("Activating to profile '{}'...", config.profile_name);

    let attr_path = format!(".#{}", config.package);

    let output = Command::new("nix")
        .args(["profile", "add", &attr_path])
        .args(["--profile-name", &config.profile_name])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run nix profile add: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Activation failed:\n{}", stderr);
    }

    println!("System activated.");
    Ok(())
}
