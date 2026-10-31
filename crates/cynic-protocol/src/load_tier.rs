use serde::{Deserialize, Serialize};

/// Classified load tier of a host, from measured CPU and memory utilisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadTier {
    Headroom,
    Normal,
    Saturated,
}
