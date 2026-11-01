//! Canonical toolchain registry model shared by `cynic-build` and `cynic-host`.
//!
//! Single source of truth for `infra/rust-toolchain.json` parsing. Both crates
//! re-export these types instead of maintaining competing models.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// The stack the workspace reserves when no crate demands more.
pub const DEFAULT_MIN_STACK: u64 = 268_435_456; // 256 MiB

/// A single crate/entry in the toolchain registry seed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolchainEntry {
    /// Canonical rustup toolchain channel (e.g. "1.95.0").
    pub toolchain: String,
    /// Fallback channel when the primary ICEs (e.g. "1.96.0").
    #[serde(default)]
    pub fallback: Option<String>,
    /// Human-readable reason for the entry/fallback.
    #[serde(default)]
    pub reason: Option<String>,
    /// Minimum RUST_MIN_STACK in bytes (stored as a decimal string).
    #[serde(default)]
    pub min_stack: Option<String>,
}

/// Versioned registry: crate_name -> ToolchainEntry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolchainRegistry {
    #[serde(default)]
    pub crates: HashMap<String, ToolchainEntry>,
}

impl ToolchainRegistry {
    pub fn entry(&self, crate_name: &str) -> Option<&ToolchainEntry> {
        self.crates.get(crate_name)
    }

    pub fn parse(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn load(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<Self>(&content).ok()
    }

    /// The stack a crate needs, or the workspace default when the registry
    /// says nothing about it.
    pub fn min_stack_for(&self, crate_name: &str) -> u64 {
        self.crates
            .get(crate_name)
            .and_then(|entry| entry.min_stack.as_deref())
            .and_then(|value| value.trim().parse::<u64>().ok())
            .unwrap_or(DEFAULT_MIN_STACK)
    }
}
