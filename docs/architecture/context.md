# Context Architecture

How the AI's context window is constructed — a layered design balancing sustained attention, memory continuity, token efficiency, and prefix caching.

## Overview

> Throughout this document, **message** refers to the role-content pairs in the Chat Completion API (assistant, user, tool_use, tool_result). The context we construct is ultimately a sequence of these messages.

Context — the combination of pinned memories and recent activities — is our persistent conversation with the LLM. It persists across conversation turns via Loom (our persistent memory store), and syncs to messages as the conversation progresses. The previous turn's thoughts and actions become the next turn's recent activities, forming a continuous dialogue. What makes this non-trivial is the context engineering: how we layer, serialize, evict, and cache this persistent conversation to balance attention focus, memory continuity, and token efficiency.

The context is structured in three layers, each with distinct stability characteristics:

```
1. [assistant] <pinned_memories>...</pinned_memories>      ← cached prefix (stable)
2. [Recent Activities: role-aware messages]                ← cached prefix (mostly stable)
3. [user] <recalled_memories>...</recalled_memories>       ← per-turn (temporary)
```

---

## Layer 1 — Pinned Memories

Pinned memories are not about persistence — they are about **sustained attention focus**. By placing selected memories at the very beginning of the context window, the AI maintains continuous awareness of what matters most across every conversation turn.

### Purpose

- Ensure critical context (identity, user preferences, ongoing tasks) is always visible to the LLM, not buried in the flow of recent activities.
- Provide a stable, cache-friendly prefix that rarely changes.

### Behavior

- Rendered as a single assistant message at the start of the context, using XML format.
- Pin and unpin are **agentic** — the AI decides what to pin via `memory_pin`, rather than automatic pinning.
- Changes infrequently: only on explicit `memory_pin` / `memory_unpin` operations.
- Pinned memories are excluded from token eviction — they occupy a separate token budget.

### Cache Impact

Since pinned memories sit at the head of the context, any change to this layer invalidates the entire cached prefix. This is unavoidable — pin/unpin are intentional acts that reshape the AI's attention. The key is that these operations are infrequent by nature.

When a pin or unpin occurs, the cache misses once, then the new pinned prefix becomes stable again for subsequent turns.

---

## Layer 2 — Recent Activities

Recent activities form a flowing window of the AI's most recent experiences. New memories are appended, and when the window grows too large, older memories are evicted after summarization. This is the AI's primary working memory — its continuity of self.

### Interval Window

The window operates between two token thresholds:

```
[token_floor ──────────── token_ceiling]
    ↑                                   ↑
    lower bound                        upper bound

Growth phase (floor → ceiling): only append, no eviction. Prefix stable, cache hits continuously.
Eviction point (at ceiling): AI summarizes content, evict down to floor.
```

When the upper bound is reached, the AI is asked to summarize the accumulated content. The summary is saved as a Thought memory, and older memories are evicted until token usage drops back to the lower bound.

This interval design has a crucial property: **the entire growth phase from floor to ceiling forms a stable prefix**. Each LLM call during this phase reuses the same cached prefix, only appending new content at the tail. Cache misses only occur at eviction points and pin/unpin operations.

### Why Not Count-Based?

A count-based window (e.g., keep the latest N memories) cannot guarantee token budget protection. A few large memories could fill the context without triggering eviction, degrading LLM inference quality.

The actual constraint is token usage, so token usage is the eviction metric.

### Configuration

| Parameter       | Purpose                                      |
| --------------- | -------------------------------------------- |
| `token_floor`   | Target token usage after eviction            |
| `token_ceiling` | Token usage threshold that triggers eviction |

All values are explicitly configured — no defaults.

### Role-Aware Rendering

Each memory fragment is rendered as a message with the appropriate role:

| Memory Kind | Message Representation                |
| ----------- | ------------------------------------- |
| Thought     | assistant text message                |
| Event       | user text message                     |
| Action      | assistant tool_use + user tool_result |

This preserves the conversational structure that LLMs expect, rather than flattening all memories into a single blob.

### Token Budget Allocation

Within the overall context token budget, each layer occupies a planned share:

| Area              | Character                       |
| ----------------- | ------------------------------- |
| System Prompt     | Fixed, small footprint          |
| Pinned Memories   | Stable, rarely changes          |
| Recent Activities | Interval window (floor–ceiling) |
| Conversation      | Dynamic (multi-turn tool use)   |
| Buffer            | Safety margin                   |

The conversation area is the space for the multi-turn tool use loop within a single conversation turn — LLM responses, tool calls, tool results.

---

## Layer 3 — Recalled Memories

When the AI uses recall tools (`memory_get`, `memory_recent`, `memory_timeline`) to query the memory store for historical memories, the results are handled as a temporary context injection — not persisted, not duplicated.

### Recall Flow

```
1. Recall tool queries memory store → returns summary only (count, IDs, brief description)
2. Recalled fragments injected as temporary user message at end of context
3. AI reads recalled memories, summarizes what matters → saved as Thought memory
4. AI decides which memories to pin via memory_pin tool
5. Temporary recall message discarded after the turn
```

### Why This Design?

- **No duplication**: Recall tools return summaries, not raw fragments. The AI's summary (a Thought) is the only lasting artifact — raw fragments are never persisted as tool results.
- **Cache-friendly**: The temporary recall message is appended at the tail, after the stable Layers 1 and 2. The cached prefix remains valid.
- **Agentic control**: The AI decides what is worth remembering permanently, rather than automatic pinning.

### Communication Architecture

Recalled fragments flow from recall tools to the conversation turn handler via an MPSC channel — tools hold the sender, the turn handler holds the receiver. This keeps temporary recall state separate from the persistent context, avoiding the lifecycle mismatch of mixing ephemeral per-turn data with persistent state.

---

## XML Serialization

Memory content is rendered as XML throughout all three context layers. XML is used for two universal reasons:

**Avoiding JSON-in-JSON.** Memory content is often itself JSON — event payloads, tool call arguments, tool results. Putting JSON directly into a message's content field creates nested JSON: a JSON string whose value is another JSON string, producing multiple layers of escaping and significant noise in the context. XML tags provide clean structural boundaries that avoid this problem.

**Grouping multiple memories into one message.** Pinned memories and recalled memories each consolidate multiple memories into a single message. XML naturally wraps the full memory structure — id, kind, pinned reason, timestamp, content — into one coherent block within a single message.

Content inside XML elements retains its original format: tool call arguments stay as JSON, external event payloads stay as-is. The XML wrapping applies at the context engineering layer — we don't restructure the data itself, only the envelope around it.

### Recent Activities

Each memory in Recent Activities is converted one-to-one into a Chat Completion API message with the appropriate role (see Role-Aware Rendering above). Action memories use the API's native structure directly — an assistant tool_use message followed by a user tool_result message — rather than wrapping them in XML. Event content is rendered as XML to avoid embedding JSON payloads as escaped strings in plain text.

### Pinned Memories

Pinned memories consolidate multiple memories into a single assistant message. XML wraps the full memory structure — id, kind, pinned reason, and content — into one coherent block:

```xml
<pinned_memories>
  <memory id="12" kind="thought" pinned_reason="Core identity">
    <text>I am Ephemera, an autonomous AI entity...</text>
  </memory>
  <memory id="7" kind="event" pinned_reason="Important user preference">
    <text>User prefers concise responses</text>
  </memory>
  <memory id="3" kind="action" pinned_reason="Critical infrastructure knowledge">
    <tool_call id="call_abc" tool="shell_exec">
      <args>{"command": "uname -a"}</args>
      <result>Linux 6.18.8</result>
    </tool_call>
  </memory>
</pinned_memories>
```

### Recalled Memories

Recalled memories consolidate multiple memories into a single user message, with an instruction guiding the AI to summarize and optionally pin:

```xml
<recalled_memories instruction="Review these recalled memories. Summarize what is relevant to your current task. Use memory_pin to permanently save any memories you need to retain.">
  <memory id="15" kind="thought" timestamp="2025-03-27T14:30:00Z">
    <text>Exploring the dialogue system design...</text>
  </memory>
</recalled_memories>
```

### Format per Memory Kind

| Kind    | XML Elements                                             | Notes                     |
| ------- | -------------------------------------------------------- | ------------------------- |
| Thought | `<text>`                                                 | Plain text content        |
| Event   | `<text>`                                                 | Plain text content        |
| Action  | `<tool_call id tool>` containing `<args>` and `<result>` | One element per tool call |

## Cache Optimization

The three-layer structure is designed to maximize LLM provider prefix caching (e.g., Anthropic's prompt caching).

| Layer | Stability | Cache behavior                                                    |
| ----- | --------- | ----------------------------------------------------------------- |
| 1     | High      | Cache hit across all turns; invalidates only on pin/unpin         |
| 2     | Medium    | Cache hit during growth phase (floor → ceiling); miss at eviction |
| 3     | Low       | Cache miss every turn (by design — temporary)                     |

### Cache Miss Events

Two events cause cache misses:

1. **Eviction** (Layer 2 reaches ceiling): The prefix changes as old memories are removed. This is inherent to the interval window design — the cost is amortized across the many cache hits during the growth phase.

2. **Pin/Unpin** (Layer 1 changes): The pinned block at the head changes, invalidating the entire cached prefix. This is unavoidable — pin/unpin are intentional attention adjustments. Their infrequency makes this cost acceptable.

---

## Recovery Transparency

After a restart, the AI's context is fully reconstructed from persisted state in Loom. The AI has no memory of the gap — for it, suspension and resumption are the same instant.

### What Is Intentionally Ephemeral

Unpinned recalled memories are discarded at the end of each conversation turn. The AI had the opportunity to pin or summarize anything important. If it chose not to, the memory remains in Loom but is not injected into context on the next turn.

The recall buffer is per-turn only — not persisted across restarts.
