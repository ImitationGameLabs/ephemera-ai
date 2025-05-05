# Architecture

## Long-Term Memory
 - **Hybrid Search**: Enables associative retrieval; typically implemented using a vector database.
 - **Time-Based Queries**: Allows querying memories based on their timestamps.
 - **Objective Metadata**: For example, the source of the memory, access frequency, timestamp, etc. Time-based queries reflect a form of time perception capability.
 - **Subjective Metadata**: The AI can attach subjective metadata to memory fragments, such as the importance of the memory or its sentiment. When reviewing memories, especially if a topic is linked to thousands of records, the system can sort and filter using these subjective tags.
 - **Complex Views**: For instance, if memories are stored sequentially by time, we might generate a graph view when performing associative recall to better connect related memories. In practice, this could mean creating a specialized index in the database.

Because of the advanced requirements time-based queries, sorting/filtering by subjective metadata, etc. I’m currently considering using a search engine for indexing rather than a vector database.

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
