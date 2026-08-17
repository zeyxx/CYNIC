//! Read-back proof layer for toolchain lifecycle.
//!
//! Produces machine-readable JSON evidence after the lifecycle completes.
//! The proof is the canonical read-back (CYNIC §3) — it is NOT a claim
//! of success, it is the observed substrate state + decision.

use crate::toolchain::observe::ToolchainObservation;
use crate::toolchain::policy::ToolchainDecision;
use serde::{Deserialize, Serialize};

/// Versioned proof schema for toolchain selection/read-back.
pub const TOOLCHAIN_PROOF_SCHEMA: &str = "cynic.runtime.toolchain_proof.v1";

/// Machine-readable proof written to target/proofs/toolchain/<crate>.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainProof {
    pub schema: String,
    pub crate_name: String,
    pub observed_version: String,
    pub observed_vendor: String,
    pub ice_detected: bool,
    pub decision: ToolchainDecision,
    pub applied_toolchain: Option<String>,
}

impl ToolchainProof {
    pub fn from(
        observed: &ToolchainObservation,
        decision: &ToolchainDecision,
    ) -> Self {
        let applied = match decision {
            ToolchainDecision::UseToolchain(t) | ToolchainDecision::UseFallback(t) => Some(t.clone()),
            _ => None,
        };

        ToolchainProof {
            schema: TOOLCHAIN_PROOF_SCHEMA.to_string(),
            crate_name: observed.crate_name.clone(),
            observed_version: observed.version.clone(),
            observed_vendor: observed.vendor.clone(),
            ice_detected: observed.ice_detected,
            decision: decision.clone(),
            applied_toolchain: applied,
        }
    }

    /// Write proof to target/proofs/toolchain directory.
    pub fn write(&self, crate_name: &str) -> Result<(), String> {
        let proof_dir = format!("target/proofs/toolchain");
        let _ = std::fs::create_dir_all(&proof_dir);
        let path = format!("{}/{}.json", proof_dir, crate_name);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("proof serialize error: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("proof write error: {}", e))?;
        Ok(())
    }

    /// Construct a proof for the Blocked state.
    pub fn blocked(crate_name: &str) -> Self {
        ToolchainProof {
            schema: TOOLCHAIN_PROOF_SCHEMA.to_string(),
            crate_name: crate_name.to_string(),
            observed_version: "unknown".to_string(),
            observed_vendor: "unknown".to_string(),
            ice_detected: false,
            decision: ToolchainDecision::Blocked("blocked before observation".to_string()),
            applied_toolchain: None,
        }
    }
}

/// Writer that persists proofs to the proof directory.
/// Separated from ToolchainProof so the proof shape stays pure.
pub struct ToolchainProofWriter;

impl ToolchainProofWriter {
    pub fn new() -> Self {
        Self
    }

    /// Write the proof to `<dir>/<crate>.json`, creating the directory.
    pub fn write_proof(&self, proof: &ToolchainProof, dir: &Path) -> Result<(), String> {
        let _ = std::fs::create_dir_all(dir);
        let path = dir.join(format!("{}.json", proof.crate_name));
        let json = serde_json::to_string_pretty(proof)
            .map_err(|e| format!("proof serialize error: {}", e))?;
        std::fs::write(&path, json)
            .map_err(|e| format!("proof write error: {}", e))?;
        Ok(())
    }
}

use std::path::Path;
