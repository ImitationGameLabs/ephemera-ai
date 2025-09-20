use ephemera_memory::{
    Manager, MemoryFragment, MemoryQuery, MemorySource, ObjectiveMetadata, Speaker,
    SubjectiveMetadata,
};
use time::OffsetDateTime;
use rig::{
    OneOrMany,
    agent::Agent,
    completion::{Completion, CompletionModel, PromptError},
    message::{AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent},
    tool::ToolSetError,
};

use tracing::debug;

pub struct Ephemera<M: CompletionModel> {
    pub chat_agent: Agent<M>,
    pub keyword_agent: Agent<M>,

    pub chat_history: Vec<Message>,
    pub memory_manager: ephemera_memory::HybridMemoryManager,
}

impl<M: CompletionModel> Ephemera<M> {
    pub async fn prompt(&mut self, prompt: String) -> anyhow::Result<String> {
        let now_timestamp = OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let memory_fragment = MemoryFragment {
            id: now_timestamp,
            content: prompt.clone(),
            subjective_metadata: SubjectiveMetadata {
                importance: 50,
                confidence: 80,
                tags: vec!["conversation".to_string()],
                notes: String::new(),
            },
            objective_metadata: ObjectiveMetadata {
                created_at: now_timestamp,
                source: MemorySource::StatementByOther(Speaker {
                    claimed_identity: "user".to_string(),
                    assessed_identity: "user".to_string(),
                }),
            },
            associations: vec![],
        };
        self.memory_manager.append(&memory_fragment).await?;
        self.chat_history.push(prompt.clone().into());

        let memories = self.recall_flow(prompt.clone()).await?;

        let mut chat_history = self.chat_history.clone();
        chat_history.push(memories.into());

        let response = self
            .prompt_test(&self.chat_agent, prompt, &mut chat_history)
            .await?;

        let response_timestamp = OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let response_memory = MemoryFragment {
            id: response_timestamp,
            content: response.clone(),
            subjective_metadata: SubjectiveMetadata {
                importance: 60,
                confidence: 90,
                tags: vec!["response".to_string()],
                notes: String::new(),
            },
            objective_metadata: ObjectiveMetadata {
                created_at: response_timestamp,
                source: MemorySource::StatementBySelf,
            },
            associations: vec![],
        };
        self.memory_manager.append(&response_memory).await?;
        self.chat_history.push(response.clone().into());

        Ok(response)
    }

    async fn tool_call(
        &self,
        agent: &Agent<M>,
        content: ToolCall,
    ) -> Result<UserContent, ToolSetError> {
        let tool_call_msg = AssistantContent::ToolCall(content.clone());
        debug!("Tool Call Msg: {:?}\n", tool_call_msg);

        let ToolCall {
            id,
            function: ToolFunction { name, arguments },
        } = content;

        let tool_result = agent.tools.call(&name, arguments.to_string()).await?;

        Ok(UserContent::tool_result(
            id,
            OneOrMany::one(ToolResultContent::text(tool_result)),
        ))
    }

    pub async fn prompt_test(
        &self,
        agent: &Agent<M>,
        prompt: impl Into<Message> + Send,
        chat_history: &mut Vec<Message>,
    ) -> Result<String, PromptError> {
        let mut current_prompt: Message = prompt.into();

        loop {
            debug!("Current Prompt: {:?}\n", current_prompt);
            let resp = agent
                .completion(current_prompt.clone(), chat_history.clone())
                .await?
                .send()
                .await?;

            chat_history.push(current_prompt.clone());

            // We only process the first choice.
            let content = resp.choice.first();
            chat_history.push(Message::Assistant {
                content: OneOrMany::one(content.clone()),
            });

            match content {
                AssistantContent::Text(text) => {
                    debug!("Intermediate Response: {:?}\n", text.text);
                    return Ok(text.text);
                }
                AssistantContent::ToolCall(content) => {
                    let tool_result = self.tool_call(agent, content).await?;

                    current_prompt = Message::User {
                        content: OneOrMany::one(tool_result),
                    };
                }
            }
        }
    }

    async fn recall_flow(&mut self, prompt: impl Into<Message> + Send) -> anyhow::Result<String> {
        debug!("Try to recall memories");

        let mut chat_history = self.chat_history.clone();

        let keywords = self
            .prompt_test(&self.keyword_agent, prompt.into(), &mut chat_history)
            .await?;

        debug!("Keywords of current chat context: {}", keywords);

        let query = MemoryQuery {
            keywords,
            time_range: None,
        };

        let memories = self.memory_manager.recall(&query).await?;
        let memories_str = serde_json::to_string(&memories)?;

        debug!("Get memories: {:?}", memories);

        Ok(memories_str)
    }
}
