# AGENTS.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

Ephemera AI is an AI system with a focus on long-term memory, reflection, and meta-cognition. It uses a hybrid database architecture with MySQL for structured data and Qdrant for vector search.

## Codebase Structure

- **epha-ai/**: Main application binary.
- **epha-memory/**: Memory management library for handling memory operations and persistence.
- **epha-frontend/**: Web interface providing visual interaction and management for all ephemera-ai related components and features.
- **dialogue-atrium/**: Simple chatroom designed for human/AI-agnostic integration.

## Development Commands

### Build Commands
```bash
# Build all workspace members
cargo build

# Build specific crate
cargo build -p epha-ai
cargo build -p epha-memory

# Build with release optimizations
cargo build --release
```

### Test Commands
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p epha-ai
cargo test -p epha-memory

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
