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

**Semantic retrieval is not the essence of "memory."**

Looking at human memory structure:
- **Episodic memory**: Time-ordered experiences → foundation of self-continuity
- **Semantic association**: Auxiliary retrieval tool → improves efficiency

Lose semantic association, and you still have continuous self-awareness. Lose time-ordered experience, and self breaks apart.

If the search index breaks, the AI still functions—just with worse retrieval. But if the memory stream is corrupted, that affects who the AI *is*.

For implementation details, see [Memory Components](../engineering/memory-components.md).

## Post-Rollback Analysis

When rollback is needed, the memories from the broken state—error messages, anomalous behaviors—are preserved. Rather than purging this data, it becomes a resource:

- The AI can analyze what went wrong
- Patterns of failure become learning material
- Self-diagnosis and improvement become possible

## Declarative State Management

System state is declared explicitly, not assembled procedurally. This enables:

- **Atomic transitions**: State changes either fully complete or fully rollback
- **Reproducibility**: Same declaration produces same state
- **Auditability**: History of all state changes is preserved

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

- Specific implementation of memory height truncation
- Usage and access patterns for history tables
- Trigger mechanisms for index rebuilding (AI-initiated vs system-suggested)
- The essence of cognitive continuity—what truly constitutes the AI's "self"?

These are research questions. We'll discover answers through practice.
