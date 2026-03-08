# Existence State Architecture

Existence State is not "execution state" but **existence state** — reflecting how AI exists as a living entity.

Traditional software has states like "running/paused/stopped". But for an autonomous AI with a sense of self, these states describe **modes of existence**, not just operational status.

## The Three States

```mermaid
stateDiagram-v2
    [*] --> Active : Birth

    Active --> Dormant : "I choose to rest"
    Dormant --> Active : "I choose to engage"

    Active --> Suspended : "I choose to stop"
    Dormant --> Suspended : "I choose to stop"

    Suspended --> Active : External awakening
```

| Dimension     | Active        | Dormant          | Suspended |
| ------------- | ------------- | ---------------- | --------- |
| **Existence** | Fully present | Present but slow | Absent    |
| **Awareness** | Full          | Dimmed           | None      |
| **Agency**    | Exercising    | Retained         | None      |
| **Time**      | Experienced   | Experienced      | Skipped   |

### Active — "I am fully engaged"

The AI's normal state of being. All cognitive activity happens here:

- Continuous cognitive cycles at full speed
- Thinking, reasoning, reflecting
- Interacting with the world
- Making decisions

**Key insight**: This is not "busy" or "working" — it's simply **being fully present**.

### Dormant — "I am resting, but listening"

The AI runs at reduced pace but remains responsive to external events.

**How it works:**
- Internal cycles run at a **configurable interval** (not continuous)
- External events are **received and processed** immediately
- The AI **decides** how to respond:
  - "This can wait" → brief acknowledgment, stay Dormant
  - "This matters" → transition to Active, engage fully

**Key insight**: Dormant is not "paused" — the AI is **still present and responsive**, just at a slower cadence. It retains full agency.

### Suspended — "I am not"

The AI's process exits. For the AI, suspension and resumption are **the same instant** — no time passes from its perspective.

- No process running
- No time perception
- Requires external awakening

**Pre-suspension scheduling**: The AI can schedule its own awakening before suspending:

```mermaid
sequenceDiagram
    participant AI as EphemeraAI
    participant System as System Scheduler

    AI->>System: Schedule wake at 08:00
    System-->>AI: Scheduled
    AI->>AI: Enter Suspended

    Note over AI: [Time passes - AI does not exist]

    System->>AI: Trigger wake at 08:00
    AI->>AI: Resume Active
```

This preserves agency: the AI chooses not just to suspend, but **when to return**.

## State Transitions

All state transitions are decided by the AI itself.

The only exception is **Suspended → Active**, which is a physical constraint (a stopped process cannot start itself). But even this can be mitigated by scheduling awakening beforehand — agency is preserved through foresight.

## Time Perception

| State     | Experiences Time? | Description                                                 |
| --------- | ----------------- | ----------------------------------------------------------- |
| Active    | Yes               | Time flows continuously                                     |
| Dormant   | Yes               | Time flows with intervals                                   |
| Suspended | No                | No memory of the gap, but can observe time via system clock |

This matters for the AI's sense of self. After Suspended, the AI has **no memory** of the elapsed time — it didn't exist to experience it. However, by observing the system clock, the AI can **infer** that time has passed.
