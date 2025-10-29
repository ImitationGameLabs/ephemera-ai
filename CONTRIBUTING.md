# Contributing to Ephemera AI

Welcome to Ephemera AI! This document will help you get started with development and contribute to the project.

## Project Overview

Ephemera AI is an AI system with long-term memory, reflection, and meta-cognition capabilities. It features a hybrid memory architecture combining MySQL for structured data and Qdrant for vector search.

## Quick Start

### Prerequisites

- **Rust** (latest stable version)
- **Docker** and **Docker Compose**
- **MySQL** and **Qdrant** (via Docker containers)
- **API Keys** for LLM and Embedding services

### 1. Clone and Setup

```bash
git clone https://github.com/EphemeraLab/ephemera-ai.git
cd ephemera-ai
```

### 2. Start Database Services

```bash
docker compose up -d
```

### 3. Configure Environment Variables

Copy the example environment file:
```bash
cp .env.example .env
```

Edit `.env` with your API keys and configurations. See [Environment Configuration](#environment-configuration) section below.

### 4. Build and Run

```bash
cargo run --bin epha-ai
```

## Service Development

### Starting Individual Services

For development, you may want to start individual microservices:

#### Loom Memory Service
```bash
cargo run --bin loom
```
- Uses `LOOM_SERVICE_PORT` environment variable for binding
- Default: `3001` (binds to `[::]:3001`)

#### Atrium Dialogue Service
```bash
cargo run --bin atrium
```
- Uses `ATRIUM_SERVICE_PORT` environment variable for binding
- Default: `3002` (binds to `[::]:3002`)

#### Using CLI Tools
```bash
# Atrium CLI client
cargo run --bin atrium-cli -- --help

# Connect to specific atrium service
cargo run --bin atrium-cli -- --server http://localhost:3002
```

### Service Configuration

Services and clients use different environment variables:

```env
# Service binding ports (for services)
LOOM_SERVICE_PORT=3001
ATRIUM_SERVICE_PORT=3002

# Client connection URLs (for clients)
LOOM_SERVICE_URL=http://localhost:3001
ATRIUM_SERVICE_URL=http://localhost:3002
```

Services bind to `[::]:port` for IPv6 compatibility.

## Environment Configuration

### Required Environment Variables

Copy `.env.example` to `.env` and fill in your actual values:

```env
# LLM Configuration (OpenAI-compatible API required)
# Model must support function calling for tool usage
# Examples: deepseek-chat, gpt-4, gpt-3.5-turbo, claude-3-sonnet
API_KEY=<your-api-key-here>
BASE_URL=https://api.deepseek.com/v1
MODEL_NAME=deepseek-chat

# Database Configuration
EPHA_MEMORY_MYSQL_URL=mysql://epha:123456@localhost:3306/epha_memory
EPHA_MEMORY_QDRANT_URL=http://localhost:6334
DIALOGUE_ATRIUM_MYSQL_URL=mysql://epha:123456@localhost:3307/dialogue_atrium

# Service Configuration
# Service ports (for services)
LOOM_SERVICE_PORT=3001
ATRIUM_SERVICE_PORT=3002

# Client URLs (for clients)
LOOM_SERVICE_URL=http://localhost:3001
ATRIUM_SERVICE_URL=http://localhost:3002

# Embedding Configuration (OpenAI-compatible API)
EMBEDDING_MODEL=embedding-3
EMBEDDING_MODEL_DIMENSIONS=2048
EMBEDDING_MODEL_URL=https://open.bigmodel.cn/api/paas/v4
EMBEDDING_MODEL_API_KEY=<your-embedding-api-key-here>
```

## Troubleshooting

### Common Issues

#### 1. Database Connection Errors
```bash
# Check Docker containers
docker ps

# View logs
docker logs <container-name>

# Restart services
docker compose restart
```

#### 3. Embedding Dimension Mismatch
- Ensure `EMBEDDING_MODEL_DIMENSIONS` matches your model's output
- Common dimensions: 2048
- Check API documentation for your chosen model

#### 4. Function Calling Errors
- Ensure your LLM model supports function calling
- DeepSeek-chat, GPT-4, Claude-3 work well
- Verify API provider supports tools

### Migration Issues

If you encounter database schema issues:

```bash
# Reset database (warning: deletes data)
docker compose down -v
docker compose up -d
```