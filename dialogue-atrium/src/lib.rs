pub mod entity;
pub mod migration;
pub mod db;
pub mod models;
pub mod handlers;
pub mod routes;
pub mod cli;

// Re-export CLI functionality
pub use cli::*;