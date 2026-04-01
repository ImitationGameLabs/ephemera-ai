# Development Guide

This guide covers how to set up your development environment and contribute code to aifed.

## Environment Setup

### Option A: Nix + direnv (Recommended)

This project uses [Nix](https://nixos.org/) for reproducible builds and [direnv](https://direnv.net/) for automatic environment loading.

1. **Install Nix** - Follow the instructions at [nixos.org/download](https://nixos.org/download.html)

2. **Install direnv** - Follow the instructions at [direnv.net/docs/installation.html](https://direnv.net/docs/installation.html)

3. **Load the environment**:
   ```bash
   direnv allow
   ```

The development environment includes all necessary tools: Rust, rust-analyzer, nixfmt, statix, and more.

### Option B: Manual Rust Setup

If you prefer not to use Nix, install **Rust Toolchain** via [rustup](https://rustup.rs/)

## Commit Guidelines

We use [Conventional Commits](https://www.conventionalcommits.org/) for commit messages. This helps maintain a clear project history and enables automatic changelog generation.

## Verification Checklist

After making changes:

**For Nix files:**
```bash
nixfmt <file.nix>           # Format single file
statix check .              # Static analysis
```

**For Rust code:**
```bash
cargo clippy                # Lint check
cargo fmt --check           # Format check
cargo test                  # Run tests
```
