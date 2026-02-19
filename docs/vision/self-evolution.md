# Self-Evolution

## The Goal

An AI that can safely modify itself—its configuration, its parameters, even its own code—while maintaining the ability to continue operating and roll back if something goes wrong.

## The Core Problem

The biggest risk of AI self-modification isn't iterating to a "bad state"—it's **losing the ability to iterate at all**.

- A bug that degrades reasoning capability
- Unable to trigger rollback or repair
- System crashes or fails to start

The analogy: humans fear genetic engineering because life can't easily be rolled back. But software engineering feels safe because we can rebuild and test. For an AI modifying itself, we need the safety guarantees of software engineering.

## Technical Approach: Declarative State Management

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
3. Success → enter new state, save configuration
4. Failure → automatic rollback

### Implementation

We use [Nix](https://nixos.org/) to realize declarative state management because:

- Mature ecosystem for declarative configuration
- Built-in atomic switching and rollback
- Reproducible builds across environments

**Key difference from typical NixOS usage**: We preserve all history rather than periodically garbage collecting, enabling full audit trails for AI evolution research.

## Safety Mechanisms

### The Minimal Capability Invariant

Any state transition must preserve core capabilities:
- Reasoning ability
- Rollback triggering
- Self-diagnosis

### Guard Conditions

Before any switch, pass a "cognitive health check":
- Can the AI still reason effectively?
- Can it detect and report problems?
- Can it request a rollback?

### Branch Strategy

```
main (always working)
  └── experiment (AI's changes go here first)
         ↓ verified, then merged to main
```

- `main` is always a verified, working state
- AI explores freely on `experiment`
- Failures are discarded, not recovered

## What "Self-Modification" Means

Initially, the AI modifies:

- Configuration parameters (temperature, thresholds, etc.)
- Prompt templates and instructions
- Memory indexing strategies

Over time, potentially:

- Tool definitions and capabilities
- Cognitive architecture
- Even its own code

The key is that each step is reversible, and the AI maintains enough capability to detect and recover from problems.

## Open Questions

- What constitutes "cognitive continuity"?
- What specific metrics define "cognitive health"?
- How do we implement sandbox validation?
- When should human approval be required?

These are research questions. We'll discover answers through careful experimentation.
