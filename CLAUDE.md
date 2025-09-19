# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Ephemera AI is a Rust-based AI system with a focus on long-term memory, reflection, and meta-cognition. It uses a hybrid database architecture with MySQL for structured data and Qdrant for vector search.

## Codebase Structure

- **ephemera-ai/**: Main application binary
- **ephemera-memory/**: Memory management library with SeaORM for MySQL
- **ephemera-mcp/**: Model Context Protocol server implementation
- **Workspace**: Cargo workspace managing all three crates

## Development Commands

### Build Commands
```bash
# Build all workspace members
cargo build

# Build specific crate
cargo build -p ephemera-ai
cargo build -p ephemera-memory
cargo build -p ephemera-mcp

# Build with release optimizations
cargo build --release
```

### Test Commands
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p ephemera-ai
cargo test -p ephemera-memory
cargo test -p ephemera-mcp

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
cargo run --bin ephemera-ai

# Run MCP server
cargo run --bin ephemera-mcp
```

## Database Architecture

- **MySQL**: Stores structured memory metadata (ephemera_memory database)
- **Qdrant**: Vector database for semantic search and embeddings
- **Connection**: Services run on localhost:3306 (MySQL) and localhost:6333 (Qdrant)

## Key Dependencies

- **SeaORM**: MySQL ORM for structured data
- **Meilisearch**: Search functionality (though README mentions Qdrant)
- **RMCP**: Model Context Protocol implementation
- **Tokio**: Async runtime
- **Tracing**: Structured logging

## Environment Configuration

Required environment variables in `.env`:
- `DEEPSEEK_API_KEY`: API key for DeepSeek LLM provider
- Database credentials are hardcoded in compose.yaml

## Development Workflow

1. Start databases: `docker compose up -d`
2. Build and run: `cargo run --bin ephemera-ai`
3. For MCP development: `cargo run --bin ephemera-mcp`
4. Use `cargo test` to verify changes
5. Run `cargo clippy` and `cargo fmt` before committing

## Important Notes

- **Avoid reading database volumes**: Claude Code should avoid reading or analyzing the `mysql_data/` and `qdrant_data/` directories as these contain database volumes that should not be committed or analyzed