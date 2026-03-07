use clap::{Parser, Subcommand};

/// Ephemera AI Deployment CLI
///
/// Manage deployment of Ephemera AI services using Nix profiles.
/// Provides atomic deployments with rollback capability.
#[derive(Parser)]
#[command(name = "epha-boot")]
#[command(about = "Deployment CLI for Ephemera AI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new deployment project
    ///
    /// Creates a new deployment directory with the default template.
    /// Example: epha-boot init
    Init {
        /// Template URL (e.g., github:ImitationGameLabs/ephemera-ai#default)
        ///
        /// For local testing: path:../ephemera-ai#default
        /// If not specified, uses {config.project_url}#{config.template}
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Update flake inputs
    ///
    /// Updates all flake inputs to their latest versions.
    Update {
        /// Specific input to update (updates all if not specified)
        #[arg(short, long)]
        input: Option<String>,
    },

    /// Build services
    ///
    /// Builds the package using nix build.
    Build,

    /// Deploy services using nix profile
    ///
    /// Performs atomic deployment with automatic rollback on failure.
    Switch,

    /// Rollback to previous deployment
    ///
    /// Reverts to the previous generation of the profile.
    Rollback,

    /// Show deployment status
    ///
    /// Displays the current contents of the profile.
    Status,

    /// List deployment history
    ///
    /// Shows the version history of the profile.
    History,
}
