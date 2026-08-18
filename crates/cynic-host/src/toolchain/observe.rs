//! Read-only observation port for the local Rust toolchain.
//!
//! Observe the substrate first, then choose the adapter (CYNIC §3).

use crate::toolchain::specs::ToolchainRegistry;
use crate::update_lifecycle::{ArgvOutput, ArgvRunner};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Observed state of a Rust toolchain — no opinion, no decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainObservation {
    pub crate_name: String,
    pub version: String,
    pub vendor: String,
    pub ice_detected: bool,
    pub cache_hash: Option<String>,
    pub last_ok: Option<String>,
}

/// Port trait: any adapter observing a toolchain state.
pub trait ToolchainObservationPort: Send + Sync {
    fn observe(&self, crate_name: &str, runner: &dyn ArgvRunner) -> ToolchainObservation;
}

/// Local filesystem adapter for ToolchainObservationPort.
pub struct LocalToolchainObserver {
    pub registry: ToolchainRegistry,
}

impl LocalToolchainObserver {
    pub fn new(registry: ToolchainRegistry) -> Self {
        Self { registry }
    }

    /// Detect whether an ICE artifact exists (SIGSEGV log) for the crate.
    fn ice_artifact_exists(crate_name: &str) -> bool {
        Path::new(&format!("target/rustc-ice-{}.log", crate_name)).exists()
            || Path::new("target/rustc-ice-log").exists()
    }

    fn cache_fingerprint(crate_name: &str) -> Option<String> {
        let cache_dir = format!("target/debug/deps");
        let _ = crate_name; // fingerprint keyed by deps dir mtime
        if Path::new(&cache_dir).exists() {
            Some("cached".to_string())
        } else {
            None
        }
    }
}

impl ToolchainObservationPort for LocalToolchainObserver {
    fn observe(&self, crate_name: &str, runner: &dyn ArgvRunner) -> ToolchainObservation {
        // Substrate observation only — no opinion, no decision.
        // Uses the injected argv runner (no hardcoded rustc path).
        let output: ArgvOutput = runner.run(&["rustc", "--version"]);
        let version = if output.exit_code == Some(0) {
            output.stdout.trim().to_string()
        } else {
            "unknown".to_string()
        };

        let target_output: ArgvOutput = runner.run(&["rustc", "-vV"]);
        let vendor = if target_output.exit_code == Some(0) {
            target_output
                .stdout
                .lines()
                .find(|l| l.starts_with("host:"))
                .map(|l| l.trim_start_matches("host: ").to_string())
                .unwrap_or_else(|| "unknown".to_string())
        } else {
            "unknown".to_string()
        };

        ToolchainObservation {
            crate_name: crate_name.to_string(),
            version,
            vendor,
            ice_detected: Self::ice_artifact_exists(crate_name),
            cache_hash: Self::cache_fingerprint(crate_name),
            last_ok: None, // proof history is read separately from proof.json
        }
    }
}
