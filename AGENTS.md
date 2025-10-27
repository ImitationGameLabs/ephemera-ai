# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

Ephemera AI is an AI system with a focus on long-term memory, reflection, and meta-cognition. It uses a hybrid database architecture with MySQL for structured data and Qdrant for vector search.

## Codebase Structure

- **epha-ai/**: Main application binary.
- **epha-agent/**: Agent system with state machine and context management capabilities.
- **psyche/**: Memory and cognition components.
  - **loom/**: Memory management library for handling memory operations and persistence.
- **epha-frontend/**: Web interface providing visual interaction and management for all ephemera-ai related components and features.
- **dialogue/**: Modular chat system designed for human/AI-agnostic integration.
  - **atrium/**: Server implementation with database and API logic.
  - **atrium-client/**: Reusable HTTP client library.
  - **atrium-cli/**: CLI tool for interacting with the chat system.

### Workspace Configuration

All workspace members are defined in:
- Root `Cargo.toml` (Rust workspace members)
- `pnpm-workspace.yaml` (Frontend workspace members, if applicable)

See these files for complete list of available projects.

## Development Commands

### Build Commands
```bash
# Build all workspace members
cargo build

# Build specific crate (examples)
cargo build -p epha-ai
cargo build -p atrium
cargo build -p loom
# See workspace configuration above for all available projects

# Build with release optimizations
cargo build --release
```

### Test Commands
```bash
# Run all tests
cargo test

# Run tests for specific crate (examples)
cargo test -p epha-ai
cargo test -p atrium
cargo test -p loom
# See workspace configuration above for all available projects

# Run specific test
cargo test --testname
```

### Linting & Formatting
```bash
# Run clippy linting
cargo clippy

# Auto-fix clippy suggestions
cargo clippy --fix

# Format code
cargo fmt
```

### Running the Application
```bash
# Start database services
docker compose up -d

# Run main application
cargo run --bin epha-ai
```

## Development Workflow

1. Start databases: `docker compose up -d`
2. Build and run: `cargo run --bin epha-ai`
3. Use `cargo test` to verify changes
4. Run `cargo clippy` and `cargo fmt` before committing

### Frontend Development

When working on frontend development in `epha-frontend/`, please read the documentation in `docs/conventions/frontend/` to understand our tech stack, color system, and development guidelines.
