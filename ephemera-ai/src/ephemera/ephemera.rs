use rig::{
    agent::Agent,
    completion::{Completion, CompletionModel, PromptError},
    message::{AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent},
    tool::ToolSetError,
    OneOrMany
};

use tracing::debug;

pub struct Ephemera<M: CompletionModel> {
    pub agent: Agent<M>,
    pub chat_history: Vec<Message>,
}

impl<M: CompletionModel> Ephemera<M> {
    pub async fn prompt(
        &mut self,
        prompt: impl Into<Message> + Send,
    ) -> Result<String, PromptError> {
        let mut current_prompt: Message = prompt.into();
        loop {
            debug!("Current Prompt: {:?}\n", current_prompt);
            let resp: rig::completion::CompletionResponse<<M as CompletionModel>::Response> = self
                .agent
                .completion(current_prompt.clone(), self.chat_history.clone())
                .await?
                .send()
                .await?;

            self.chat_history.push(current_prompt.clone());

            // We only process the first choice.
            let content = resp.choice.first();
            self.chat_history.push(Message::Assistant { 
                content: OneOrMany::one(content.clone()) 
            });

            match content {
                AssistantContent::Text(text) => {
                    debug!("Intermediate Response: {:?}\n", text.text);
                    return Ok(text.text);
                }
                AssistantContent::ToolCall(content) => {
                    let tool_result =  self.tool_call(content).await?;

                    current_prompt = Message::User {
                        content: OneOrMany::one(tool_result),
                    };
                }
            }
        }
    }

    async fn tool_call(&mut self, content: ToolCall) -> Result<UserContent, ToolSetError> {
        let tool_call_msg = AssistantContent::ToolCall(content.clone());
        debug!("Tool Call Msg: {:?}\n", tool_call_msg);

        self.chat_history.push(Message::Assistant {
            content: OneOrMany::one(tool_call_msg),
        });

        let ToolCall {
            id,
            function: ToolFunction { name, arguments },
        } = content;

        let tool_result = self.agent.tools
            .call(&name, arguments.to_string())
            .await?;

        Ok(UserContent::tool_result(
            id,
            OneOrMany::one(ToolResultContent::text(tool_result)),
        ))
    }
}
