pub struct Config {
    /// Profile name for nix profile operations
    pub profile_name: String,

    /// Package attribute path to build/install
    pub package: String,

    /// Default template name for init command
    pub template: String,

    /// Project URL for fetching templates
    pub project_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile_name: "ephemera-ai".to_string(),
            package: "default".to_string(),
            template: "default".to_string(),
            project_url: "github:ImitationGameLabs/ephemera-ai".to_string(),
        }
    }
}
