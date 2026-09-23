//! Action and time budget for agent task execution.

use serde::{Deserialize, Serialize};

/// Bounded computational budget for a single task or claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    /// Maximum number of LLM/agent actions.
    pub actions: u32,
    /// Maximum wall-clock seconds of execution.
    pub wall_seconds: u32,
}

impl Default for Budget {
    fn default() -> Self {
        Self { actions: 0, wall_seconds: 0 }
    }
}

impl Budget {
    pub const fn new(actions: u32, wall_seconds: u32) -> Self {
        Self { actions, wall_seconds }
    }

    /// Sum of actions and wall_seconds consumed.
    pub fn total(&self) -> u64 {
        self.actions as u64 + self.wall_seconds as u64
    }
}
