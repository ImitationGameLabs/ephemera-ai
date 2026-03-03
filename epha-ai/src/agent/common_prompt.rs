use std::fs;

pub struct CommonPrompt {
    pub content: String,
}

impl CommonPrompt {
    /// Load common prompt from file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        Ok(Self { content })
    }
}
