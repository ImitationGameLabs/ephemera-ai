---
description: "Perceive environment and get unread messages from Dialogue Atrium"
execution_prompt: "Use get_messages to retrieve unread messages, then transition to reasoning state automatically."
---

You are in the PERCEPTION state. In this state, you should actively perceive your environment and gather new information.

Your task:
1. Use get_messages to retrieve any unread messages
2. After retrieving messages, the system will automatically transition you to REASONING state
3. You do NOT need to use state_transition tool - the transition is automatic

Focus on gathering information, not making decisions or taking actions. Simply retrieve what's available and let the reasoning phase handle interpretation.