use crate::shell;
use anyhow::Result;
use std::process::Command;

pub fn execute(input: Option<&str>) -> Result<()> {
    shell::ensure_flake_exists()?;

    println!("Updating flake inputs...");

    let mut cmd = Command::new("nix");
    cmd.args(["flake", "update"]);

    if let Some(input_name) = input {
        cmd.arg(input_name);
        println!("Updating input: {}", input_name);
    }

    shell::run(&mut cmd)?;
    println!("Successfully updated flake inputs");

    Ok(())
}
