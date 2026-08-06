//! Human-in-the-Loop Operational Supervision Policy.
//!
//! Governs model switching and runtime mutations under sovereign authority.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SupervisionPolicy {
    /// Auto-switch for lightweight models (< 4GB VRAM) with direct proof notification;
    /// Require interactive confirmation for heavy models (>= 4GB VRAM or 9B+ param class).
    #[default]
    HybridIntentAndCapacity,
    /// Unconditional Human-in-the-loop confirmation for any model switch.
    AlwaysConfirm,
    /// Unconditional automatic switch with direct proof notification.
    AlwaysAuto,
}

impl SupervisionPolicy {
    /// Check whether switching to `target_alias` with `target_size_bytes` requires
    /// interactive operator confirmation under this policy.
    pub fn requires_confirmation(&self, target_size_bytes: Option<u64>, target_alias: &str) -> bool {
        match self {
            Self::AlwaysConfirm => true,
            Self::AlwaysAuto => false,
            Self::HybridIntentAndCapacity => {
                let size = target_size_bytes.unwrap_or(0);
                let heavy_by_bytes = size >= 4 * 1024 * 1024 * 1024;
                let heavy_by_alias = target_alias.contains("9b")
                    || target_alias.contains("27b")
                    || target_alias.contains("35b");
                heavy_by_bytes || heavy_by_alias
            }
        }
    }

    pub fn confirmation_reason(&self, target_size_bytes: Option<u64>, target_alias: &str) -> Option<String> {
        if self.requires_confirmation(target_size_bytes, target_alias) {
            let size_gb = target_size_bytes.map(|s| s as f64 / (1024.0 * 1024.0 * 1024.0)).unwrap_or(0.0);
            Some(format!(
                "Heavy model transition detected (alias='{}', size={:.2} GB). Requires explicit operator confirmation under HybridIntentAndCapacity policy.",
                target_alias, size_gb
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_policy_requires_confirmation_for_large_models() {
        let policy = SupervisionPolicy::HybridIntentAndCapacity;
        assert!(policy.requires_confirmation(Some(5 * 1024 * 1024 * 1024), "qwythos-9b-q4"));
        assert!(policy.requires_confirmation(None, "bonsai-27b-q1"));
        assert!(!policy.requires_confirmation(Some(1 * 1024 * 1024 * 1024), "minicpm-1b-q8"));
        assert!(!policy.requires_confirmation(Some(2 * 1024 * 1024 * 1024), "qwen35-2b-q4"));
    }
}
