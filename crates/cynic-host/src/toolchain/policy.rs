//! Data-centric reconciliation policy for toolchain lifecycle.
//!
//! Full Compass: reconcile (registry seed + proof history)
//! + (observed substrate state + build outcome) -> ToolchainState ->
//! ToolchainTransition (bounded action) -> proof append.
//!
//! States model the *full* lifecycle, not just the happy path.

use crate::toolchain::observe::ToolchainObservation;
use crate::toolchain::specs::{ToolchainProofHistory, ToolchainRegistry};
use serde::{Deserialize, Serialize};

/// Current state of the toolchain lifecycle for a crate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolchainState {
    Unknown,
    Bootstrap,
    Converged,
    UpgradeReady,
    Upgraded,
    IceDetected,
    FallbackApplied,
    FallbackOk,
    FallbackFailed,
    Blocked,
    LearntPromoted,
}

/// Bounded action to transition between states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolchainTransition {
    Noop,
    SetToolchain(String),
    Halt { reason: String },
    PromoteFallback {
        from: String,
        to: String,
    },
}

/// Input to the reconciliation compass: substrate + outcome + history.
pub struct ToolchainReconcileInput<'a> {
    pub crate_name: &'a str,
    pub registry: &'a ToolchainRegistry,
    pub history: &'a ToolchainProofHistory,
    pub observed: &'a ToolchainObservation,
    /// Outcome of the last build attempt (exit code + ICE detection).
    pub build_ok: bool,
    /// ICE artifact path if rustc crashed (e.g. target/rustc-ice-log).
    pub ice_artifact: Option<String>,
}

/// Reconcile current + observed + last build outcome into next state + transition.
///
/// Compass: registry + proof history (desired) + build outcome (observed)
/// -> ToolchainState (reconcile) -> ToolchainTransition (bounded action)
pub fn reconcile_toolchain_full(
    input: &ToolchainReconcileInput,
    current_state: ToolchainState,
) -> (ToolchainState, ToolchainTransition) {
    let crate_name = input.crate_name;
    let entry = input.registry.entry(crate_name);

    match current_state {
        ToolchainState::Unknown => handle_unknown(input, entry),
        ToolchainState::Converged => handle_converged(input, entry),
        ToolchainState::IceDetected => handle_ice_detected(input),
        ToolchainState::UpgradeReady => handle_upgrade_ready(input, entry),
        ToolchainState::Upgraded => handle_upgraded(input),
        ToolchainState::FallbackOk => handle_fallback_ok(input),
        ToolchainState::Bootstrap
        | ToolchainState::FallbackApplied
        | ToolchainState::FallbackFailed
        | ToolchainState::LearntPromoted
        | ToolchainState::Blocked => handle_terminal(input, entry),
    }
}

fn handle_unknown(
    input: &ToolchainReconcileInput,
    entry: Option<&crate::toolchain::specs::ToolchainEntry>,
) -> (ToolchainState, ToolchainTransition) {
    if let Some(e) = entry {
        if input.build_ok {
            (
                ToolchainState::Converged,
                ToolchainTransition::SetToolchain(e.toolchain.clone()),
            )
        } else if input.ice_artifact.is_some() {
            match &e.fallback {
                Some(fb) => (
                    ToolchainState::IceDetected,
                    ToolchainTransition::SetToolchain(fb.clone()),
                ),
                None => (
                    ToolchainState::Blocked,
                    ToolchainTransition::Halt {
                        reason: "ICE with no fallback declared".into(),
                    },
                ),
            }
        } else {
            (ToolchainState::Bootstrap, ToolchainTransition::Noop)
        }
    } else {
        // No registry seed — check proof history for a learned toolchain.
        if input.build_ok {
            if let Some(learned) = input.history.last_ok(input.crate_name) {
                (
                    ToolchainState::LearntPromoted,
                    ToolchainTransition::PromoteFallback {
                        from: "none".to_string(),
                        to: learned.to_string(),
                    },
                )
            } else {
                (ToolchainState::Bootstrap, ToolchainTransition::Noop)
            }
        } else {
            (
                ToolchainState::Blocked,
                ToolchainTransition::Halt {
                    reason: "no registry entry for crate".into(),
                },
            )
        }
    }
}

fn handle_converged(
    input: &ToolchainReconcileInput,
    entry: Option<&crate::toolchain::specs::ToolchainEntry>,
) -> (ToolchainState, ToolchainTransition) {
    if input.ice_artifact.is_some() {
        let fb = entry.and_then(|e| e.fallback.as_ref());
        let learned = input.history.last_ok(input.crate_name);
        let fallback = fb
            .map(|s| s.to_string())
            .or_else(|| learned.map(|l| l.to_string()));
        match fallback {
            Some(f) => (
                ToolchainState::IceDetected,
                ToolchainTransition::SetToolchain(f),
            ),
            None => (
                ToolchainState::Blocked,
                ToolchainTransition::Halt {
                    reason: "ICE, no fallback or learned toolchain".into(),
                },
            ),
        }
    } else if input.build_ok {
        if let Some(e) = entry {
            if e.toolchain != input.observed.version {
                (
                    ToolchainState::UpgradeReady,
                    ToolchainTransition::SetToolchain(e.toolchain.clone()),
                )
            } else {
                (ToolchainState::Converged, ToolchainTransition::Noop)
            }
        } else {
            (ToolchainState::Converged, ToolchainTransition::Noop)
        }
    } else {
        (
            ToolchainState::Blocked,
            ToolchainTransition::Halt {
                reason: "build failed (non-ICE)".into(),
            },
        )
    }
}

fn handle_ice_detected(input: &ToolchainReconcileInput) -> (ToolchainState, ToolchainTransition) {
    if input.build_ok {
        (ToolchainState::FallbackOk, ToolchainTransition::Noop)
    } else if input.ice_artifact.is_some() {
        (
            ToolchainState::FallbackFailed,
            ToolchainTransition::Halt {
                reason: "fallback also ICE'd".into(),
            },
        )
    } else {
        (ToolchainState::FallbackApplied, ToolchainTransition::Noop)
    }
}

fn handle_upgrade_ready(
    input: &ToolchainReconcileInput,
    entry: Option<&crate::toolchain::specs::ToolchainEntry>,
) -> (ToolchainState, ToolchainTransition) {
    if input.build_ok {
        (ToolchainState::Upgraded, ToolchainTransition::Noop)
    } else if input.ice_artifact.is_some() {
        match entry.and_then(|e| e.fallback.clone()) {
            Some(f) => (
                ToolchainState::IceDetected,
                ToolchainTransition::SetToolchain(f),
            ),
            None => (
                ToolchainState::Blocked,
                ToolchainTransition::Halt {
                    reason: "upgrade ICE, no fallback".into(),
                },
            ),
        }
    } else {
        // Upgrade failed (non-ICE) — rollback to last converged.
        (ToolchainState::Converged, ToolchainTransition::Noop)
    }
}

fn handle_upgraded(input: &ToolchainReconcileInput) -> (ToolchainState, ToolchainTransition) {
    if input.build_ok {
        (ToolchainState::Converged, ToolchainTransition::Noop)
    } else {
        (
            ToolchainState::Blocked,
            ToolchainTransition::Halt {
                reason: "post-upgrade validation failed".into(),
            },
        )
    }
}

fn handle_fallback_ok(input: &ToolchainReconcileInput) -> (ToolchainState, ToolchainTransition) {
    let seed_tc = input
        .registry
        .entry(input.crate_name)
        .map(|e| e.toolchain.clone())
        .unwrap_or_else(|| "unknown".to_string());
    // The promoted toolchain is the fallback that got applied and verified.
    // That is the observed version (after rustup override set <fallback>),
    // falling back to proof history (last_ok), then the seed.
    let promoted = if input.observed.version != "unknown" {
        input.observed.version.clone()
    } else {
        input
            .history
            .last_ok(input.crate_name)
            .map(String::from)
            .unwrap_or_else(|| seed_tc.clone())
    };
    (
        ToolchainState::LearntPromoted,
        ToolchainTransition::PromoteFallback {
            from: seed_tc,
            to: promoted,
        },
    )
}

fn handle_terminal(
    input: &ToolchainReconcileInput,
    entry: Option<&crate::toolchain::specs::ToolchainEntry>,
) -> (ToolchainState, ToolchainTransition) {
    if let Some(e) = entry {
        if input.build_ok {
            (
                ToolchainState::Converged,
                ToolchainTransition::SetToolchain(e.toolchain.clone()),
            )
        } else {
            (
                ToolchainState::Blocked,
                ToolchainTransition::Halt {
                    reason: "terminal state reached without convergence".into(),
                },
            )
        }
    } else {
        (
            ToolchainState::Blocked,
            ToolchainTransition::Halt {
                reason: "no registry entry".into(),
            },
        )
    }
}

// ---------------------------------------------------------------------------
// Legacy single-decision shim (backward compat for callers needing a
// ToolchainDecision). Delegates to the full Compass starting from Converged.
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ToolchainDecision {
    Converged,
    UseToolchain(String),
    UseFallback(String),
    UseLearned(String),
    Blocked(String),
}

pub fn reconcile_toolchain(
    crate_name: &str,
    registry: &ToolchainRegistry,
    history: &ToolchainProofHistory,
    observed: &ToolchainObservation,
) -> ToolchainDecision {
    let input = ToolchainReconcileInput {
        crate_name,
        registry,
        history,
        observed,
        build_ok: !observed.ice_detected,
        ice_artifact: if observed.ice_detected {
            Some("target/rustc-ice-log".into())
        } else {
            None
        },
    };

    let (state, _transition) = reconcile_toolchain_full(&input, ToolchainState::Converged);

    match state {
        ToolchainState::Converged => ToolchainDecision::Converged,
        ToolchainState::Bootstrap => match registry.entry(crate_name) {
            Some(e) => ToolchainDecision::UseToolchain(e.toolchain.clone()),
            None => ToolchainDecision::Blocked("no entry".into()),
        },
        ToolchainState::IceDetected | ToolchainState::FallbackApplied => {
            let fb_owned: Option<String> = registry
                .entry(crate_name)
                .and_then(|e| e.fallback.clone())
                .or_else(|| history.last_ok(crate_name).map(|s| s.to_string()));
            match fb_owned {
                Some(f) => {
                    if history.last_ok(crate_name).map(|s| s == f.as_str()).unwrap_or(false) {
                        ToolchainDecision::UseLearned(f)
                    } else {
                        ToolchainDecision::UseFallback(f)
                    }
                }
                None => ToolchainDecision::Blocked("ICE, no fallback".into()),
            }
        }
        _ => ToolchainDecision::Blocked("unresolved state".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolchain::observe::ToolchainObservation;
    use crate::toolchain::specs::{
        ToolchainEntry, ToolchainProofEvent, ToolchainProofHistory, ToolchainRegistry,
    };
    use std::collections::HashMap;

    fn registry(t: &str, fb: Option<&str>) -> ToolchainRegistry {
        let mut crates = HashMap::new();
        crates.insert(
            "test-crate".to_string(),
            ToolchainEntry {
                toolchain: t.to_string(),
                fallback: fb.map(|s| s.to_string()),
                reason: None,
                min_stack: None,
            },
        );
        ToolchainRegistry { crates }
    }

    fn history_learned(name: &str, tc: &str) -> ToolchainProofHistory {
        let mut entries = HashMap::new();
        entries.insert(
            name.to_string(),
            vec![ToolchainProofEvent {
                toolchain: tc.to_string(),
                compile_ok: true,
                ice_detected: false,
                timestamp: None,
            }],
        );
        ToolchainProofHistory { entries }
    }

    fn obs(ice: bool, version: &str) -> ToolchainObservation {
        ToolchainObservation {
            crate_name: "test-crate".to_string(),
            version: version.to_string(),
            vendor: "x86_64-unknown-linux-gnu".to_string(),
            ice_detected: ice,
            cache_hash: None,
            last_ok: None,
        }
    }

    fn input<'a>(
        reg: &'a ToolchainRegistry,
        hist: &'a ToolchainProofHistory,
        o: &'a ToolchainObservation,
        build_ok: bool,
        ice: Option<&'a str>,
    ) -> ToolchainReconcileInput<'a> {
        ToolchainReconcileInput {
            crate_name: "test-crate",
            registry: reg,
            history: hist,
            observed: o,
            build_ok,
            ice_artifact: ice.map(|s| s.to_string()),
        }
    }

    #[test]
    fn unknown_crate_converges_when_build_ok() {
        let reg = registry("1.96.0", Some("1.95.0"));
        let hist = ToolchainProofHistory::default();
        let o = obs(false, "1.96.0");
        let i = input(&reg, &hist, &o, true, None);
        let (state, trans) = reconcile_toolchain_full(&i, ToolchainState::Unknown);
        assert_eq!(state, ToolchainState::Converged);
        assert_eq!(trans, ToolchainTransition::SetToolchain("1.96.0".to_string()));
    }

    #[test]
    fn unknown_crate_ice_uses_fallback() {
        let reg = registry("1.96.0", Some("1.95.0"));
        let hist = ToolchainProofHistory::default();
        let o = obs(true, "1.96.0");
        let i = input(&reg, &hist, &o, false, Some("target/rustc-ice"));
        let (state, _) = reconcile_toolchain_full(&i, ToolchainState::Unknown);
        assert_eq!(state, ToolchainState::IceDetected);
    }

    #[test]
    fn converged_with_ice_transitions_to_fallback() {
        let reg = registry("1.96.0", Some("1.95.0"));
        let hist = history_learned("test-crate", "1.96.0");
        let o = obs(true, "1.96.0");
        let i = input(&reg, &hist, &o, false, Some("target/rustc-ice"));
        let (state, trans) = reconcile_toolchain_full(&i, ToolchainState::Converged);
        assert_eq!(state, ToolchainState::IceDetected);
        assert_eq!(trans, ToolchainTransition::SetToolchain("1.95.0".to_string()));
    }

    #[test]
    fn fallback_ok_promotes_to_learnt() {
        let reg = registry("1.96.0", Some("1.95.0"));
        let hist = ToolchainProofHistory::default();
        let o = obs(false, "1.95.0");
        let i = input(&reg, &hist, &o, true, None);
        let (state, trans) = reconcile_toolchain_full(&i, ToolchainState::FallbackOk);
        assert_eq!(state, ToolchainState::LearntPromoted);
        assert_eq!(
            trans,
            ToolchainTransition::PromoteFallback {
                from: "1.96.0".to_string(),
                to: "1.95.0".to_string(),
            }
        );
    }

    #[test]
    fn fallback_failed_when_both_ice() {
        let reg = registry("1.96.0", Some("1.95.0"));
        let hist = ToolchainProofHistory::default();
        let o = obs(true, "1.95.0");
        let i = input(&reg, &hist, &o, false, Some("target/rustc-ice"));
        let (state, _) = reconcile_toolchain_full(&i, ToolchainState::IceDetected);
        assert_eq!(state, ToolchainState::FallbackFailed);
    }

    #[test]
    fn unknown_crate_promotes_learned_without_seed() {
        let reg = ToolchainRegistry::default();
        let hist = history_learned("test-crate", "1.95.0");
        let o = obs(false, "1.95.0");
        let i = input(&reg, &hist, &o, true, None);
        let (state, _) = reconcile_toolchain_full(&i, ToolchainState::Unknown);
        assert_eq!(state, ToolchainState::LearntPromoted);
    }

    #[test]
    fn unknown_crate_no_seed_no_history_blocks() {
        let reg = ToolchainRegistry::default();
        let hist = ToolchainProofHistory::default();
        let o = obs(false, "unknown");
        let i = input(&reg, &hist, &o, false, None);
        let (state, _) = reconcile_toolchain_full(&i, ToolchainState::Unknown);
        assert_eq!(state, ToolchainState::Blocked);
    }
}
