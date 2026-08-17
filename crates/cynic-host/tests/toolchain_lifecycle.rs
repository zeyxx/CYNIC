//! Integration test: full toolchain lifecycle ICE cycle (data-centric).
//!
//! Simulates the complete compass path:
//!   Unknown → Bootstrap → Converged → (ICE) → IceDetected → FallbackApplied
//!   → FallbackOk → LearntPromoted → Converged
//!
//! All substrate observation is faked via a map argv runner — no real rustc
//! needed (CYNIC §5: closed harness for proof).

use cynic_host::toolchain::observe::{ToolchainObservation, ToolchainObservationPort};
use cynic_host::toolchain::policy::{
    reconcile_toolchain_full, ToolchainDecision, ToolchainState, ToolchainTransition,
    ToolchainReconcileInput,
};
use cynic_host::toolchain::specs::{ToolchainProofHistory, ToolchainRegistry};
use cynic_host::update_lifecycle::{ArgvOutput, ArgvRunner};
use std::collections::HashMap;

/// Fake runner that returns scripted outputs for rustup/rustc argv.
struct FakeRunner {
    script: HashMap<String, ArgvOutput>,
}

impl FakeRunner {
    fn new() -> Self {
        Self {
            script: HashMap::new(),
        }
    }

    fn with_script(script: HashMap<String, ArgvOutput>) -> Self {
        Self { script }
    }
}

impl ArgvRunner for FakeRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        let key = argv.join(" ");
        self.script.get(&key).cloned().unwrap_or(ArgvOutput {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: None,
        })
    }
}

fn registry() -> ToolchainRegistry {
    ToolchainRegistry {
        crates: HashMap::from([(
            "test-crate".to_string(),
            cynic_host::toolchain::ToolchainEntry {
                toolchain: "1.96.0".to_string(),
                fallback: Some("1.95.0".to_string()),
                reason: Some("ICE fallback".to_string()),
                min_stack: None,
            },
        )]),
    }
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

fn reconcile_input<'a>(
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
fn ice_cycle_full_compass() {
    let reg = registry();
    let mut hist = ToolchainProofHistory::default();

    // Step 1: Unknown -> Converged (first build OK with seed toolchain 1.96.0)
    let o = obs(false, "1.96.0");
    let input = reconcile_input(&reg, &hist, &o, true, None);
    let (state, trans) = reconcile_toolchain_full(&input, ToolchainState::Unknown);
    assert_eq!(state, ToolchainState::Converged);
    assert_eq!(trans, ToolchainTransition::SetToolchain("1.96.0".to_string()));

    // Record: 1.96.0 is converged
    hist.append("test-crate", "1.96.0".to_string(), true);

    // Step 2: Converged -> IceDetected -> Fallback (fallback 1.95.0)
    let o_ice = obs(true, "1.96.0");
    {
        let input = reconcile_input(&reg, &hist, &o_ice, false, Some("target/rustc-ice-log"));
        let (state, trans) = reconcile_toolchain_full(&input, ToolchainState::Converged);
        assert_eq!(state, ToolchainState::IceDetected);
        assert_eq!(trans, ToolchainTransition::SetToolchain("1.95.0".to_string()));
    }

    // Step 3: IceDetected -> FallbackOk (fallback 1.95.0 compiles OK)
    let o_fb = obs(false, "1.95.0");
    let (state, _) = {
        let input = reconcile_input(&reg, &hist, &o_fb, true, None);
        reconcile_toolchain_full(&input, ToolchainState::IceDetected)
    };
    assert_eq!(state, ToolchainState::FallbackOk);

    // Record: 1.95.0 is now known-good
    hist.append("test-crate", "1.95.0".to_string(), true);

    // Step 4: FallbackOk -> LearntPromoted (fallback promoted via history)
    let (state, trans) = {
        let input = reconcile_input(&reg, &hist, &o_fb, true, None);
        reconcile_toolchain_full(&input, ToolchainState::FallbackOk)
    };
    assert_eq!(state, ToolchainState::LearntPromoted);
    assert_eq!(
        trans,
        ToolchainTransition::PromoteFallback {
            from: "1.96.0".to_string(),
            to: "1.95.0".to_string(),
        }
    );

    // Step 5: LearntPromoted -> Converged (next build OK with learned toolchain)
    let (state, _) = {
        let input = reconcile_input(&reg, &hist, &o_fb, true, None);
        reconcile_toolchain_full(&input, ToolchainState::LearntPromoted)
    };
    assert_eq!(state, ToolchainState::Converged);

    // Verify history has learned the fallback
    assert_eq!(hist.last_ok("test-crate"), Some("1.95.0"));
}

#[test]
fn fallback_failure_blocked_when_no_seed() {
    let reg = ToolchainRegistry::default();
    let mut hist = ToolchainProofHistory::default();
    let o = obs(true, "unknown");
    let input = reconcile_input(&reg, &hist, &o, false, Some("target/rustc-ice"));
    let (state, trans) = reconcile_toolchain_full(&input, ToolchainState::Unknown);
    assert_eq!(state, ToolchainState::Blocked);
    match trans {
        ToolchainTransition::Halt { reason } => {
            assert!(
                reason.contains("fallback") || reason.contains("registry"),
                "unexpected halt reason: {reason}"
            );
        }
        _ => panic!("expected Halt transition"),
    }
}

#[test]
fn history_persists_and_reloads() {
    let reg = registry();
    let mut hist = ToolchainProofHistory::default();
    hist.append("test-crate", "1.96.0".to_string(), true);
    hist.append("test-crate", "1.95.0".to_string(), true);

    // Save + reload round-trip
    let path = std::env::temp_dir().join("cynic-tc-history-test.json");
    hist.save(&path).unwrap();
    let reloaded = ToolchainProofHistory::load(&path).unwrap();
    assert_eq!(reloaded.last_ok("test-crate"), Some("1.95.0"));
    std::fs::remove_file(&path).ok();
}
