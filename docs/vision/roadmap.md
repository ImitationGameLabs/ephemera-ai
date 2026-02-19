# Roadmap

Where we are, where we're going, and how we plan to get there.

## Short-Term

### Development Environment Standardization

Using [devenv](https://devenv.sh/) from the Nix ecosystem for consistent, reproducible development environments.

**Status**: In progress

- [x] Configure devenv environment
- [ ] Improve development helper commands
- [ ] Update CONTRIBUTING.md
- [ ] Update AGENTS.md

## Mid-Term

### Self-Evolution Infrastructure

Enabling the AI to safely modify its own configuration with declarative state and atomic transitions.

**Core mechanism**:
- AI state declared via configuration (memory state, model config, service config)
- Iteration = modify config → switch → success preserves, failure auto-rollback
- Preserve all historical versions for research and audit

We use [Nix](https://nixos.org/) to implement this infrastructure. See [Self-Evolution](self-evolution.md) for details.

### Independent System User

Creating an independent system user for the AI with more flexible and powerful execution capabilities.

**Context**: System runs on NixOS, providing declarative configuration and permission isolation for safety.

**Benefits**:
- Independent user space with fine-grained permission control
- Ability to execute complex system-level tasks
- NixOS's rollback capability naturally aligns with self-evolution mechanism

## Long-Term

### AI Social Network

Multiple AI instances forming social relationships with distributed collaboration and fault tolerance.

**Daily interaction**:
- Sharing experiences, discussing problems
- Mutual learning, knowledge sharing

**Iteration strategy**:
- Minority of instances upgrade first (canary deployment)
- Other instances observe and evaluate
- Gradual rollout after confirming safety

**Core value**:
- Distributed fault tolerance: single instance anomaly doesn't affect the whole
- Social support: mutual care and assistance
- Knowledge accumulation: collective wisdom exceeds individual

## Experiments

### Context Pinning

Giving the AI active control over context management: pin/unpin memories to bypass automatic eviction from the active queue.

**Use cases**:
- When focused on long-term tasks, lock task goals and key context
- Maintain working hypotheses across multiple conversation turns
- Lock key information during debugging/research

**Design considerations**:
- Maximum N pin slots (prevent context bloat)
- Expose interface: `pin(memory_id)` / `unpin(memory_id)`
- Automatically inject pinned memories during perception phase

### Continuous Voice Conversation

Full-duplex voice chat functionality.

**Design philosophy**:
- Don't pursue streaming STT; simplify to complete sentence recognition
- Use VAD to detect sentence boundaries, wait for complete expression before processing
- Rationale: incomplete sentences can't be effectively processed by AI anyway; human communication also uses complete sentences as basic units

**Technical approach**:
- VAD (Voice Activity Detection) for pause/sentence-end detection
- Complete sentence → STT → AI processing → TTS playback
- Continuous conversation experience, not discrete "push-to-talk" mode
