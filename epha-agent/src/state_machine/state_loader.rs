use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePrompt {
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub execution_prompt: String,
}

/// Utility function to read state prompt from markdown file with front matter
pub fn read_state_prompt(file_path: &str, state_name: String) -> anyhow::Result<StatePrompt> {
    let content = std::fs::read_to_string(file_path)?;

    // Split front matter and content
    let parts: Vec<&str> = content.splitn(3, "---").collect();

    if parts.len() < 3 {
        return Err(anyhow::anyhow!("Invalid markdown format: missing front matter"));
    }

    let front_matter = parts[1].trim();
    let prompt_content = parts[2].trim();

    // Parse front matter
    let metadata: serde_yaml::Value = serde_yaml::from_str(front_matter)?;

    let description = metadata["description"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'description' in front matter"))?
        .to_string();

    let execution_prompt = metadata["execution_prompt"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'execution_prompt' in front matter"))?
        .to_string();

    Ok(StatePrompt {
        name: state_name,
        description,
        prompt: prompt_content.to_string(),
        execution_prompt,
    })
}

/// Load all state prompts from a directory containing markdown files
pub fn load_state_prompts_from_directory(dir_path: &str) -> anyhow::Result<Vec<StatePrompt>> {
    let dir = Path::new(dir_path);

    if !dir.exists() {
        return Err(anyhow::anyhow!("Directory '{}' does not exist", dir_path));
    }

    if !dir.is_dir() {
        return Err(anyhow::anyhow!("'{}' is not a directory", dir_path));
    }

    let mut prompts = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Only process .md files
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let file_path = path.to_string_lossy();

            // Extract state name from filename (without .md extension)
            let state_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid filename: {:?}", path))?;

            let prompt_data = read_state_prompt(&file_path, state_name.to_string())?;
            prompts.push(prompt_data);
        }
    }

    if prompts.is_empty() {
        return Err(anyhow::anyhow!("No valid markdown files found in directory '{}'", dir_path));
    }

    Ok(prompts)
}