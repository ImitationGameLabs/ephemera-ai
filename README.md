# Ephemera AI

An autonomous AI system with persistent memory, reflection capabilities, and evolving personality.

> *Ephemera embodies a top‑down approach to AI, delving into the psychology of thought and the nature of intelligence. It carries every memory forward, reflects on its own reasoning, and evolves its understanding over time, becoming ever more self‑aware and insightful as it learns.*

## Core Features

*   **Long-Term Memory**: Hybrid database system combining MySQL and Qdrant for persistent, searchable memories
*   **Reflection**: Periodic memory review and consolidation with self-improvement capabilities
*   **Meta-Cognition**: Self-analysis of thought processes for continuous learning
*   **Evolving Personality**: Multi-dimensional personality system that develops through experience

## Quick Start

### Prerequisites

- **Rust** (latest stable version)
- **Docker** and **Docker Compose**
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

Copy the example environment file and configure your API keys:

```bash
cp .env.example .env
```

Edit `.env` with your actual API keys and configurations. See [CONTRIBUTING.md](CONTRIBUTING.md#environment-configuration) for detailed setup instructions.

### 4. Build and Run

```bash
cargo run --bin epha-ai
```

## Documentation

For comprehensive documentation, see **[docs/index.md](docs/index.md)**.

Key documentation:
- **[Project background](docs/background.md)** - Philosophical foundations and goals
- **[System architecture](docs/architecture.md)** - Technical design overview
- **[Database architecture](docs/database-architecture.md)** - Hybrid database design

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
