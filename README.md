# Ephemera AI （Early Stage）

## Introduction

Ephemera AI is a project that aims to create a more sophisticated and human-like artificial intelligence. Unlike traditional chatbots that have limited memory, Ephemera is designed to have a persistent, long-term memory that evolves. The project's name is an ironic nod to the idea that even with vast knowledge, the AI should remain "humble."

> *Ephemera embodies a top‑down approach to AI, delving into the psychology of thought and the nature of intelligence. It carries every memory forward, reflects on its own reasoning, and evolves its understanding over time, becoming ever more self‑aware and insightful as it learns.*

For further reading, see [BACKGROUND.md](docs/BACKGROUND.md)

## Core Features

*   **Long-Term Memory:** Ephemera uses a hybrid database system to store its "memories." It combines a **MySQL** database for structured information (like timestamps, sources, and importance of memories) with a **Qdrant** vector database for semantic search. This allows the AI to not only recall facts but also to find memories with similar meanings.
*   **Reflection:** The AI can periodically review its memories, consolidate them, and even update its "feelings" or "opinions" about them. This process helps it learn and also acts as a "forgetting" mechanism to prune unimportant information.
*   **Meta-Cognition:** Ephemera has a special mode where it can analyze its own thought processes. This is like a "debug mode" for its own reasoning, allowing it to learn from its mistakes and improve its thinking over time.
*   **Personality and Beliefs:** The AI is designed to develop a personality and a set of beliefs, desires, and intentions (BDI). These are shaped by its memories and experiences, and in turn, influence its behavior and responses.

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
