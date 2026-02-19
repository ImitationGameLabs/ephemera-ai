# Architecture

Technical documentation for Ephemera AI's system design.

> **Note**: Our architecture is actively evolving. This section documents both our current implementation and the direction we're heading.

## Current vs Evolving

- **[Current Implementation](current.md)**: The hybrid storage system in production today
- **[Evolving Architecture](evolving.md)**: The declarative state system we're building toward

## Core Components

### [Personality System](personality.md)

Multi-dimensional personality model that develops through experience, using multiple complementary frameworks as "different lenses."

### [Agent Orchestration](agent-orchestration.md)

Configuration-driven agent orchestration enabling runtime flexibility, performance monitoring, and self-optimization.

## Design Principles

1. **Memory as Life Experience**: The memory stream is the foundation of self-continuity
2. **Retrieval as Tool**: Vector search is an external resource, not part of core identity
3. **Integrity First**: Memory systems must protect authenticity, not just enable retrieval
4. **Evolution Safe**: All changes must be reversible with automatic rollback

## Component Overview

```
epha-ai (main program)
├── epha-agent (state machine framework)
├── loom-client → loom (memory service: MySQL + Qdrant)
└── atrium-client → atrium (dialogue service: MySQL)
```

### State Machine Loop

1. **Perception** → Receive messages
2. **Recall** → Retrieve memories
3. **Reasoning** → State decision
4. **Output** → Send response
