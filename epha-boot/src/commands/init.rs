use crate::shell;
use anyhow::Result;
use std::process::Command;

pub fn execute(template_url: &str) -> Result<()> {
    println!("Initializing deployment project in current directory...");

    // Check if nix is available
    shell::run(Command::new("nix").arg("--version"))?;

    // Initialize from template in current directory
    shell::run(
        Command::new("nix")
            .args(["flake", "init", "-t", template_url]),
    )?;

    println!("Successfully initialized deployment project!");
    println!("\nNext steps:");
    println!("  1. Edit config.nix with your configuration");
    println!("  2. Run 'epha-boot build' to verify configuration");
    println!("  3. Run 'epha-boot switch' to deploy");

    Ok(())
}
