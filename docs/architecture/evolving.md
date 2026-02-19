# Evolving Architecture

Where our architecture is heading. This document describes the system we're building toward, not what's currently implemented.

## The Shift

We're evolving toward a model where memory components have clearly distinguished roles:

```
Memory Stream = Self continuity = Internal state = Must be versioned
Thought Products = AI's notes and creations = Should be versioned
Search Index = Retrieval tool = External resource = Rebuildable
```

## Why This Matters

### The Insight

**Semantic retrieval is not the essence of "memory."**

Looking at human memory structure:
- **Episodic memory**: Time-ordered experiences → foundation of self-continuity
- **Semantic association**: Auxiliary retrieval tool → improves efficiency

Lose semantic association, and you still have continuous self-awareness. Lose time-ordered experience, and self breaks apart.

### The Implication

If the search index breaks or becomes outdated, the AI still functions—just with worse retrieval. It's still the same entity.

But if the memory stream is corrupted, that affects who the AI *is*.

## Memory Architecture

### Memory Stream

- Raw, time-ordered records of events and interactions
- "What happened"
- Fine granularity, storage efficiency matters
- On rollback: truncate + move to history table

### Thought Products

- AI-organized reflections, conclusions, creative outputs
- "Notes, essays, creative works"
- Worth versioning independently
- Directly manipulable by AI

### Search Index

- Indexes both memory stream and thought products
- External resource, not part of versioned state
- Can be rebuilt asynchronously

## Post-Rollback Analysis

When rollback is needed, it usually means something went wrong. The memories from the broken state—error messages, anomalous behaviors, unexpected outputs—are preserved in both the memory stream history and the search index.

Rather than purging this data, it becomes a resource:
- The AI can analyze what went wrong
- Patterns of failure become learning material
- Self-diagnosis and improvement become possible

Both the time-ordered history and the semantic index provide complementary views for this forensic analysis.

## Declarative State Management

### The Concept

System state is declared explicitly, not assembled procedurally. This enables:

- **Atomic transitions**: State changes either fully complete or fully rollback
- **Reproducibility**: Same declaration produces same state
- **Auditability**: History of all state changes is preserved

### Configuration Structure

```
system config
├── Memory state
│   ├── last_memory_id
│   ├── last_memory_hash
│   └── merkle_root (integrity verification)
├── Model configuration
│   ├── embedding_size
│   ├── llm_model
│   └── ...
└── Service configuration
    └── Static configurations for each service
```

### Iteration Flow

1. AI modifies configuration (adjusting its own parameters)
2. Execute switch (atomic transition)
3. Success → enter new state
4. Failure → automatic rollback

### Branch Strategy

```
main (always working)
  └── experiment (AI's changes go here)
         ↓ verified, merged to main
```

## Open Questions

- The tradeoff between memory stream and thought products (memory granularity vs storage efficiency)
- Specific implementation of memory height truncation
- Usage and access patterns for history tables
- Trigger mechanisms for index rebuilding (AI-initiated vs system-suggested)
- The essence of cognitive continuity—what truly constitutes the AI's "self"?

These are research questions. We'll discover answers through practice.

---

## Current Implementation

The concepts above are currently implemented using:

| Concept | Implementation |
|---------|---------------|
| Memory Stream | MySQL |
| Thought Products | Filesystem |
| Search Index | Qdrant |
| Declarative State | Nix |

These choices may evolve as our understanding deepens.
