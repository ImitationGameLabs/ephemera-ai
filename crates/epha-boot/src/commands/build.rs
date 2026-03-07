use crate::{config::Config, shell};
use anyhow::{bail, Result};
use std::process::Command;

pub fn execute(config: &Config) -> Result<()> {
    shell::ensure_flake_exists()?;

    println!("Building {}...", config.package);

    let attr_path = format!(".#{}", config.package);

    let output = Command::new("nix")
        .args(["build", &attr_path, "--no-link", "--print-out-paths"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run nix build: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Build failed:\n{}", stderr);
    }

    let store_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("Built: {}", store_path);
    Ok(())
}
