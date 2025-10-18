use std::fs;

pub struct CommonPrompt {
    pub content: String,
}

impl CommonPrompt {
    /// Load common prompt from file, directly returning CommonPrompt instance
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self { content })
    }

    /// Get combined prompt (common + state specific)
    pub fn combine_with_state_prompt(&self, state_prompt: &str) -> String {
        format!("{}\n\n{}", self.content, state_prompt)
    }
}