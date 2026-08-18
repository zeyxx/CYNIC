//! Portable facade for toolchain lifecycle.
//!
//! Implements the bottom of the CYNIC coherence pattern:
//!   desired state (registry + proof history) -> observe -> reconcile
//!   -> bounded action -> read-back proof
//!
//! Adapters (LocalArgvRunner, local filesystem observer) are injected;
//! replacing the substrate adapter must not require rewriting the policy.

use crate::toolchain::observe::{
    LocalToolchainObserver, ToolchainObservation, ToolchainObservationPort,
};
use crate::toolchain::policy::{ToolchainDecision, reconcile_toolchain};
use crate::toolchain::proof::{ToolchainProof, ToolchainProofWriter};
use crate::toolchain::specs::{ToolchainProofHistory, ToolchainRegistry};
use crate::update_lifecycle::{ArgvOutput, ArgvRunner};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// High-level facade coordinating observe -> reconcile -> apply -> proof.
///
/// This is the Compass orchestrator: registry seed + proof history (desired)
/// reconciled against observed substrate state (observed) -> decision ->
/// bounded argv action -> read-back proof appended to history.
pub struct ToolchainLifecycle {
    pub observer: LocalToolchainObserver,
}

impl ToolchainLifecycle {
    pub fn new(observer: LocalToolchainObserver) -> Self {
        Self { observer }
    }

    /// Run the full lifecycle for a crate: observe -> reconcile -> apply -> proof.
    ///
    /// Returns the decision + read-back proof. The caller injects `RUSTUP_TOOLCHAIN`
    /// env var from `decision` before invoking cargo (bounded action surface).
    pub fn run(
        &self,
        crate_name: &str,
        registry_path: &Path,
        history_path: &Path,
        runner: &dyn ArgvRunner,
    ) -> Result<ToolchainLifecycleReport, String> {
        // --- 1. Desired state: registry seed + proof history ---
        let registry = match ToolchainRegistry::load(registry_path) {
            Some(r) => r,
            None => {
                return Ok(ToolchainLifecycleReport {
                    crate_name: crate_name.to_string(),
                    observed: self.observer.observe(crate_name, runner),
                    decision: ToolchainDecision::Blocked("registry load failed".into()),
                    applied: None,
                    proof: ToolchainProof::blocked(crate_name),
                    residual_risk: None,
                });
            }
        };

        let history = ToolchainProofHistory::load(history_path).unwrap_or_default();

        // --- 2. Observe: read-only substrate (rustc --version, ICE artifacts) ---
        let observed = self.observer.observe(crate_name, runner);

        // --- 3. Reconcile: pure fn — registry + history + observed -> decision ---
        let decision = reconcile_toolchain(crate_name, &registry, &history, &observed);

        // --- 4. Bounded action (only on explicit decision) ---
        let applied = match &decision {
            ToolchainDecision::UseToolchain(t)
            | ToolchainDecision::UseFallback(t)
            | ToolchainDecision::UseLearned(t) => {
                // Bounded mutation: rustup override set <toolchain>.
                // The caller sets RUSTUP_TOOLCHAIN=<t> before cargo; we only record.
                Some(runner.run(&["rustup", "override", "set", t.as_str()]))
            }
            _ => None,
        };

        // --- 5. Read-back proof + append history ---
        let proof = ToolchainProof::from(&observed, &decision);
        let writer = ToolchainProofWriter::new();
        writer.write_proof(&proof, Path::new("target/proofs/toolchain"))?;

        // Append to history (data-centric feedback loop)
        let mut updated_history = history.clone();
        updated_history.append(crate_name, observed.version.clone(), !observed.ice_detected);
        updated_history.save(history_path)?;

        Ok(ToolchainLifecycleReport {
            crate_name: crate_name.to_string(),
            observed: observed.clone(),
            decision: decision.clone(),
            applied,
            proof,
            residual_risk: linker_risk(crate_name),
        })
    }
}

/// Detect linker/native-dep environment issues that block test execution
/// but not pure-fn compilation. CYNIC §3: unknown is data.
fn linker_risk(crate_name: &str) -> Option<String> {
    std::fs::read_to_string(format!("target/toolchain-linker-risk-{}.txt", crate_name))
        .ok()
        .filter(|s| !s.is_empty())
}

/// Read-back report after the lifecycle completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainLifecycleReport {
    pub crate_name: String,
    pub observed: ToolchainObservation,
    pub decision: ToolchainDecision,
    pub applied: Option<ArgvOutput>,
    pub proof: ToolchainProof,
    /// Residual risk: linker/native dep environment issues.
    pub residual_risk: Option<String>,
}

// Re-export for convenience (CYNIC §6: one canonical decision source per concern).
pub use crate::toolchain::policy::ToolchainDecision as _;
