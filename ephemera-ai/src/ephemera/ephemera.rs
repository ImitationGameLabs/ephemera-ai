use rig::{
    agent::Agent,
    completion::{CompletionModel, Completion, PromptError},
    message::{AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent},
    OneOrMany,
};

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
            println!("Current Prompt: {:?}\n", current_prompt);
            let resp = self
                .agent
                .completion(current_prompt.clone(), self.chat_history.clone())
                .await?
                .send()
                .await?;

            let mut final_text = None;

            for content in resp.choice.into_iter() {
                match content {
                    AssistantContent::Text(text) => {
                        println!("Intermediate Response: {:?}\n", text.text);
                        final_text = Some(text.text.clone());
                        self.chat_history.push(current_prompt.clone());
                        let response_message = Message::Assistant {
                            content: OneOrMany::one(AssistantContent::text(&text.text)),
                        };
                        self.chat_history.push(response_message);
                    }
                    AssistantContent::ToolCall(content) => {
                        self.chat_history.push(current_prompt.clone());
                        let tool_call_msg = AssistantContent::ToolCall(content.clone());
                        println!("Tool Call Msg: {:?}\n", tool_call_msg);

                        self.chat_history.push(Message::Assistant {
                            content: OneOrMany::one(tool_call_msg),
                        });

                        let ToolCall {
                            id,
                            function: ToolFunction { name, arguments },
                        } = content;

                        let tool_result =
                            self.agent.tools.call(&name, arguments.to_string()).await?;

                        current_prompt = Message::User {
                            content: OneOrMany::one(UserContent::tool_result(
                                id,
                                OneOrMany::one(ToolResultContent::text(tool_result)),
                            )),
                        };

                        final_text = None;
                        break;
                    }
                }
            }

            if let Some(text) = final_text {
                return Ok(text);
            }
        }
    }
}
