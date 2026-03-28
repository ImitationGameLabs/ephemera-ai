# Architecture

Technical documentation for Ephemera AI's system design.

## Core Concepts

### [Existence State](existence-state.md)

Three existence states: Active, Dormant, and Suspended. Not execution states, but **modes of being**.

### [Memory Kinds](memory-kinds.md)

Classification of memory fragments by **agency** — who initiated the activity:
- **Thought** — AI internal (reasoning, planning)
- **Action** — AI external (tool calls, execution results)
- **Event** — External world (dialogue, timer, system events)

### [Context Architecture](context.md)

How the AI's context window is constructed from memory fragments — summarize-first recall, XML serialization, and three-layer chat history optimized for prefix caching.

### [Event System](event-system.md)

The Herald-Agora architecture for event delivery:
- **Heralds** — Protocol adapters that convert external services to Ephemera AI events
- **Agora** — Event hub with SQLite persistence and at-least-once delivery