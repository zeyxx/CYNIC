//! Pure registry shapes for toolchain lifecycle (data-centric).
//!
//! The registry at infra/rust-toolchain.json is a *seed* — a starting point
//! that gets projected into the live runtime via proof history. The Compass
//! reconciles registry (declarative seed) + proof history (observed learning)
//! + substrate (rustc --version) into a decision.
//!
//! This is NOT a static table: fallback channels are promoted via feedback
//! when ICE is observed and the fallback compiles.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A single crate/entry in the toolchain registry seed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainEntry {
    /// Canonical rustup toolchain channel (e.g. \"1.96.0\").
    pub toolchain: String,
    /// Fallback channel when the primary ICEs (e.g. \"1.95.0\").
    #[serde(default)]
    pub fallback: Option<String>,
    /// Human-readable reason for the entry/fallback.
    #[serde(default)]
    pub reason: Option<String>,
    /// Minimum RUST_MIN_STACK in bytes.
    #[serde(default)]
    pub min_stack: Option<String>,
}

/// Versioned registry: crate_name -> ToolchainEntry.
/// Loaded from infra/rust-toolchain.json (observé via port).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolchainRegistry {
    pub crates: std::collections::HashMap<String, ToolchainEntry>,
}

impl ToolchainRegistry {
    pub fn entry(&self, crate_name: &str) -> Option<&ToolchainEntry> {
        self.crates.get(crate_name)
    }

    /// Load registry seed from infra/rust-toolchain.json.
    pub fn load(path: &Path) -> Option<ToolchainRegistry> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<ToolchainRegistry>(&content).ok()
    }
}

/// Proof history: accumulated learning from observed ICE + successful builds.
/// This is the dynamic projection layer — the registry seed gets patched
/// here when a fallback is discovered to work.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolchainProofHistory {
    /// crate_name -> list of (toolchain, compile_ok, ice_detected) observations.
    pub entries: std::collections::HashMap<String, Vec<ToolchainProofEvent>>,
}

/// One observed event in the proof history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainProofEvent {
    pub toolchain: String,
    pub compile_ok: bool,
    pub ice_detected: bool,
    pub timestamp: Option<String>,
}

impl ToolchainProofHistory {
    /// Promote a fallback channel to \"verified working\" if it compiled
    /// without ICE. This grows the registry seed dynamically.
    pub fn promote_fallback(&mut self, crate_name: &str, fallback: &str) {
        self.entries
            .entry(crate_name.to_string())
            .or_default()
            .push(ToolchainProofEvent {
                toolchain: fallback.to_string(),
                compile_ok: true,
                ice_detected: false,
                timestamp: None,
            });
    }

    /// Returns the last known-good toolchain for the crate (from proof history).
    pub fn last_ok(&self, crate_name: &str) -> Option<&str> {
        self.entries.get(crate_name).and_then(|events| {
            events
                .iter()
                .rev()
                .find(|e| e.compile_ok && !e.ice_detected)
                .map(|e| e.toolchain.as_str())
        })
    }

    /// Append an observed build outcome to the history (feedback loop).
    pub fn append(&mut self, crate_name: &str, toolchain: String, compile_ok: bool) {
        let ice_detected = !compile_ok; // simplified: build fail == ICE here
        self.entries
            .entry(crate_name.to_string())
            .or_default()
            .push(ToolchainProofEvent {
                toolchain,
                compile_ok,
                ice_detected,
                timestamp: Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs().to_string())
                        .unwrap_or_default(),
                ),
            });
    }

    /// Load proof history from a JSON file.
    pub fn load(path: &Path) -> Option<ToolchainProofHistory> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<ToolchainProofHistory>(&content).ok()
    }

    /// Persist the history to a JSON file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("history save serialize: {}", e))?;
        let parent = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("history save mkdir: {}", e))?;
        std::fs::write(path, json).map_err(|e| format!("history save write: {}", e))?;
        Ok(())
    }

    /// Resolve the on-disk path for the history file.
    pub fn path(crate_name: &str) -> PathBuf {
        PathBuf::from(format!("target/proofs/toolchain/{}.history.json", crate_name))
    }
}
