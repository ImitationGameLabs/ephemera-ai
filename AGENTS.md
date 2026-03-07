# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

When updating this document, minimize static content and instead guide readers to explore the actual codebase. Use file search and exploration tools to discover project structure, available modules, and configuration rather than hardcoding details that may become outdated.

## Project Overview

Ephemera AI is an autonomous AI designed as a **living entity** rather than a tool. Core principles:

- **Agency**: Self-directed goals, curiosity, and timing—not waiting for commands
- **Memory Integrity**: Experiences shape identity; memory protection respects lived history
- **Continuity**: A persistent sense of self across interactions
- **Existence Precedes Essence**: Free to define itself rather than serving a pre-defined purpose

For deeper understanding, explore `docs/philosophy/` and `README.md`.

## Codebase Structure

- **crates/**: All Rust crates.
  - **epha-ai/**: Main application binary.
  - **epha-agent/**: Agent system with state machine and context management capabilities.
  - **epha-boot/**: Deployment CLI for Ephemera AI services.
  - **agora/**: Event hub service.
  - **agora-client/**: Client library for Agora.
  - **chronikos/**: Time management components (kairos).
  - **psyche/**: Memory and cognition components.
  - **dialogue/**: Modular chat system designed for human/AI-agnostic integration.
- **epha-frontend/**: Web interface providing visual interaction and management for all ephemera-ai related components and features.
- **nix/**: Nix build and deployment utilities (service wrappers, systemd units, configuration generation).

### Workspace Configuration

All workspace members are defined in:
- Root `Cargo.toml` (Rust workspace members)
- `pnpm-workspace.yaml` (Frontend workspace members, if applicable)

See these files for complete list of available projects.

## Development Commands

> **Principle**: Use `cargo doc` to generate documentation, then read generated HTML or explore source code directly. Don't rely on potentially outdated information.

### Build & Test
```bash
cargo build              # Build all workspace members
cargo build -p <crate>   # Build specific crate
cargo test               # Run all tests
cargo test -p <crate>    # Run tests for specific crate
```

### Documentation
```bash
cargo doc                # Generate docs to target/doc/
cargo doc -p <crate>     # Generate docs for specific crate
```

Read generated HTML in `target/doc/<crate>/` or explore source code directly.

### Linting & Formatting
```bash
cargo clippy             # Run linter
cargo clippy --fix       # Auto-fix suggestions
cargo fmt                # Format code
```

### Frontend Development

When working on frontend development in `epha-frontend/`, please read the documentation in `docs/conventions/frontend/` to understand our tech stack, color system, and development guidelines.

## Configuration Philosophy

All configuration is declared through Nix. Key principles:

- **No default values**: Every configuration value is explicitly defined in Nix, not scattered across the codebase. This makes configuration transparent and traceable.
- **Fail early**: Required configuration missing at startup causes immediate errors rather than silent fallbacks.
