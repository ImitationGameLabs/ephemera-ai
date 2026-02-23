# Memory Design

Three-layer memory architecture design. See [Evolving Architecture](../architecture/evolving.md) for the architectural philosophy.

## Memory Stream

Status: **Simplified implementation completed**

Raw, time-ordered records of events and interactions:
- "What happened"
- Fine granularity, storage efficiency matters
- On rollback: truncate + move to history table
- **Keep it minimal**: only id, content, timestamp, source

The stream should be an immutable event log—append-only, no modifications. Any AI processing of memories (importance ratings, tags, associations) belongs elsewhere.

See [Memory Stream Simplification](memory-stream-simplification.md) for the design evolution.

## Thought Products

Status: **In design**

AI-organized reflections, conclusions, creative outputs:
- "Notes, essays, creative works"
- Worth versioning independently
- Directly manipulable by AI
- **Implemented as filesystem**, not a database table

### Why Filesystem Over Structured Storage

| Aspect | Filesystem | Database Table |
|--------|------------|----------------|
| Format freedom | markdown, json, code... | fixed schema |
| Versioning | git native | requires extra design |
| Human readable | open directly | need query |
| AI manipulation | existing tools | new APIs needed |
| Organization | free directory structure | predefined |

The AI already has file system tools (read, write, edit, grep, glob). Let it organize its thoughts freely.

**Not yet integrated into AI operations.**

## Search Index

Status: **In design**

Indexes both memory stream and thought products:
- External resource, not part of versioned state
- Can be rebuilt asynchronously

### Search Philosophy

Semantic retrieval is primary; structured filtering is secondary.

When AI searches for memories, it thinks in concepts ("breakfast moments"), not numbers ("importance > 200"). Therefore:

- `importance`, `confidence`, `tags` → stored as Qdrant payload
- Used as **search boost factors**, not independent filter conditions
- Precise numeric queries (e.g., "importance = 187") are likely overengineering

This means we don't need a separate SQL table for thought product metadata—the vector index is sufficient for retrieval, with metadata influencing ranking rather than serving as filter criteria.

**Not yet integrated into AI operations.**
