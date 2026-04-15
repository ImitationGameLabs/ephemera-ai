use std::fs;

pub struct GroundingPrompt {
    pub content: String,
}

impl GroundingPrompt {
    /// Load grounding prompt from file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self { content })
    }

    /// Append additional context from a file, separated by a horizontal rule.
    pub fn with_append_file(mut self, path: &str) -> anyhow::Result<Self> {
        let append = fs::read_to_string(path)?;
        self.content.push_str("\n\n---\n\n");
        self.content.push_str(&append);
        Ok(self)
    }
}
