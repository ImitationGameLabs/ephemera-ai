# Current Implementation

> **Note**: This documents the current hybrid storage system. See [Evolving Architecture](evolving.md) for where we're heading.

## Long-Term Memory

Core capabilities:

- **Hybrid Search**: Enables associative retrieval of related memories
- **Time-Based Queries**: Query memories by timestamp, enabling temporal reasoning
- **Objective Metadata**: Source, access frequency, timestamps—enables time perception
- **Subjective Metadata**: AI-attached importance, sentiment—enables prioritized recall
- **Complex Views**: Graph views for associative recall, connecting related memories

**Current implementation**: MySQL for structured metadata and time-based queries; Qdrant for vector-based semantic search.

Memory management will be encapsulated behind an interface, allowing the underlying implementation to be swapped out as the project evolves.

When a reasoning process is triggered, relevant memories are fetched and injected into the current context, including:
 - **Cold Memory**: Retrieved on demand via search.
 - **Hot Memory**: The most recent *N* turns of conversation or thought.

## Reflection
 - **Triggers**: Can be time-based or proactively initiated.
 - **Updating Subjective Metadata**: Importance ratings, sentiment toward a memory, etc., which may evolve over time or experiences.
 - **Memory Consolidation**: Review and organize fragmented memories, then append the refined content as new memory fragments. Update or suppress original fragments (e.g., duplicates) to reduce noise in future reasoning.

Organizing fragments and updating subjective metadata simulates a forgetting mechanism, which is critical for efficient learning.

## Meta-Cognition
Meta-cognitive processes must be explicitly triggered to enter this state, reducing memory pressure during normal operation. In meta-cognition mode, the AI records its entire reasoning process in detail, akin to enabling a debug mode on its own thinking. The outcomes of meta-cognition are appended to memory and influence future associative recall, reflection, and reasoning.

## Instincts, Personality, and BDI (Belief, Desire, Intention)
Aside from memory, certain persistent states like personality and BDI must be maintained. Why distinguish personality from persistent memory? Personality influences every incoming prompt and the AI’s reasoning style, acting as a more stable context compared to the rolling window of recent memory. BDI, by contrast, is more volatile and changes more frequently.

Both personality and BDI are continuously shaped by memory and reflection.

Which processes constitute instincts? For example, memory retrieval during reasoning, reflection triggers, meta-cognition activation, and personality updates.

Belief overlaps somewhat with memory.

Relationship Graph:
 - Instincts -influence-> Behavior
 - Memory -shapes-> Personality
 - Personality -influences-> BDI
 - BDI -influences-> Behavior
 - Behavior -creates-> Memory

## Context-Aware Interaction
Use various MCPs (Model Context Protocols) to interact with the environment. By implementing an MCP registration and discovery center, the AI can autonomously connect to whichever protocols it needs based on its goals and intentions.
