
# Ephemera AI （Early Stage）
## Introduction
Ephemera embodies a top‑down approach to AI, delving into the psychology of thought and the nature of intelligence. It carries every memory forward, reflects on its own reasoning, and evolves its understanding over time, becoming ever more self‑aware and insightful as it learns.

For further reading, see [BACKGROUND.md](docs/BACKGROUND.md)

## Features
 - [ ] Long-Term Memory
 - [ ] Time Perception
 - [ ] Reflection
 - [ ] Meta Cognition
 - [ ] Personality
 - [ ] Context-Aware Interaction
 - [ ] Multimodal I/O

For implementation details of these features, please refer to [ARCHITECTURE.md](docs/ARCHITECTURE.md)

## Quick Start

Edit .env file
```
DEEPSEEK_API_KEY=<your-api-key>
```

Run
```bash
# Run database components (MySQL and Qdrant) in background.
docker compose up -d

cargo run --bin ephemera-ai
```

Currently, there is no built-in support for different model providers. If you wish to use a different LLM provider, you can modify the relevant code sections in [ephemera-ai/src/main.rs](ephemera-ai/src/main.rs).

## Database Architecture
Ephemera uses a hybrid database approach with MySQL for structured metadata and Qdrant for vector-based semantic search. For detailed architecture information, see [DATABASE_ARCHITECTURE.md](docs/DATABASE_ARCHITECTURE.md).

## Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md).
