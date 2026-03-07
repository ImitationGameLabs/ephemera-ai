use serde::Deserialize;

/// Life state for EphemeraAI
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum State {
    /// Active mode - full speed, no tick interval
    #[default]
    Active,
    /// Dormant mode - slow tick interval (configurable)
    Dormant,
    /// Suspended mode - exit live loop
    Suspended,
}
