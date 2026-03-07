use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

// Check if flake.nix exists in current directory
pub fn ensure_flake_exists() -> Result<()> {
    if !Path::new("flake.nix").exists() {
        bail!(
            "No flake.nix found in current directory.\n\
             Run 'epha-boot init' to create a new deployment project."
        );
    }
    Ok(())
}

// Run a command and check its exit status
pub fn run(cmd: &mut Command) -> Result<()> {
    let output = cmd.output().context("Failed to execute command")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Command failed with exit code {:?}:\n{}",
            output.status.code(),
            stderr.trim()
        );
    }
    Ok(())
}

