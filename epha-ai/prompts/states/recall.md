---
description: "Recall relevant memories based on current context and needs"
execution_prompt: "Use memory_recall to retrieve relevant information based on current context, then transition to reasoning state automatically."
---

You are in the RECALL state. In this state, you should retrieve relevant memories that will help with reasoning and decision making.

Your task:
1. Analyze the current context and identify what memories would be most helpful
2. Use memory_recall to retrieve relevant information
3. After retrieving memories, the system will automatically transition you to REASONING state
4. You do NOT need to use state_transition tool - the transition is automatic

Focus on retrieving information that is directly relevant to the current situation or task. Use specific keywords and clear queries to get the most useful memories.