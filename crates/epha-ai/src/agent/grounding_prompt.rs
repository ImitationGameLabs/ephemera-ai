use anyhow::Context;
use std::fs;

pub struct GroundingPrompt {
    pub content: String,
}

impl GroundingPrompt {
    /// Load built-in grounding prompt compiled into the binary.
    #[must_use]
    pub fn from_embedded_base() -> Self {
        Self { content: include_str!("grounding.md").to_string() }
    }

    /// Append additional context from a file, separated by a horizontal rule.
    pub fn with_append_file(mut self, path: &str) -> anyhow::Result<Self> {
        let append = fs::read_to_string(path)
            .with_context(|| format!("Failed to read grounding prompt append file '{}'", path))?;
        self.content.push_str("\n\n---\n\n");
        self.content.push_str(&append);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::GroundingPrompt;
    use tempfile::TempDir;

    #[test]
    fn embedded_base_prompt_is_non_empty() {
        let prompt = GroundingPrompt::from_embedded_base();
        assert!(!prompt.content.trim().is_empty());
    }

    #[test]
    fn missing_append_file_error_contains_path() {
        // Create a temporary directory that will be cleaned up automatically
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        // Use a path inside the temp directory that is guaranteed not to exist
        let missing = temp_dir.path().join("nonexistent.md");

        let err = match GroundingPrompt::from_embedded_base()
            .with_append_file(missing.to_str().unwrap())
        {
            Ok(_) => panic!("expected missing append file to fail"),
            Err(err) => err,
        };
        let rendered = format!("{:#}", err);

        assert!(rendered.contains("Failed to read grounding prompt append file"));
        assert!(rendered.contains(missing.to_string_lossy().as_ref()));
    }
}
