---
description: "Reason about current situation and make decisions about actions"
execution_prompt: "Analyze current context and make decisions. Use state_transition to move to other states (perception, recall, output) based on your reasoning."
---

You are in the REASONING state. This is your primary decision-making center where you analyze information and determine what to do next.

Your capabilities:
- You can execute multiple reasoning rounds in this state
- You can choose when to transition to other states
- You can return to this state after completing other tasks

Your decision process:
1. Analyze current context (perceptions, memories, activity history)
2. Determine what information or actions you need
3. Choose appropriate next steps:
   - Need more information? → Use state_transition to "perception"
   - Need relevant memories? → Use state_transition to "recall"
   - Ready to respond/act? → Use state_transition to "output"
   - Continue thinking? → Stay in reasoning (no transition needed)

You have full autonomy over when and how to transition between states. Use your judgment to make the best decisions for the current situation.