pub mod entity;
pub mod handlers;
pub mod manager;

use crate::services::system_configs::manager::SystemConfigManager;
use std::sync::Arc;

/// Application state for system configs service
#[derive(Clone)]
pub struct AppState {
    pub system_config_manager: Arc<SystemConfigManager>,
}
