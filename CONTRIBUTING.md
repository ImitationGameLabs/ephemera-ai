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
cargo run --bin ephemera-ai
```

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
MYSQL_URL=mysql://ephemera_ai:123456@localhost:3306/ephemera_memory
QDRANT_URL=http://localhost:6334

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