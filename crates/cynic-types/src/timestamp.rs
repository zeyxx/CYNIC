//! Wall-clock timestamp (Unix seconds).

use serde::{Deserialize, Serialize};

/// Unix timestamp in seconds. Wraps a `u64` for type safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(pub u64);

impl Timestamp {
    pub const fn new(secs: u64) -> Self {
        Timestamp(secs)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Elapsed seconds since this timestamp.
    pub fn elapsed_since(&self) -> Option<u64> {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs().saturating_sub(self.0))
    }
}

impl From<u64> for Timestamp {
    fn from(secs: u64) -> Self {
        Timestamp(secs)
    }
}

impl From<Timestamp> for u64 {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Timestamp(0)
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
