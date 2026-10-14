//! Pure retention decision tests: eligibility, ordering, and fail-closes.
//!
//! No filesystem here — the adapter owns observation. These tests pin the
//! pure rule: what is eligible under pressure and under the hard cap, in
//! what order, and what is never eligible.

use super::{
    policy::reconcile_build_cache,
    types::{
        BUILD_CACHE_OBSERVATION_SCHEMA, BuildCacheDecision, BuildCacheDesiredState,
        BuildCacheObservation, BuildCachePathState, EphemeralEntry, EphemeralKind,
    },
};

const MAX_AGE_MS: u64 = 7 * 24 * 3_600_000;
const GRACE_MS: u64 = 3_600_000;

fn desired() -> BuildCacheDesiredState {
    BuildCacheDesiredState {
        node_id: "builder-a".to_string(),
        state_root: "/state".to_string(),
        temporary_root: "/tmp".to_string(),
        canonical_target: "/state/targets".to_string(),
        minimum_available_bytes: 100,
        reclaim_profile: "debug".to_string(),
        preserved_profiles: vec!["release".to_string()],
        noncanonical_cache_paths: Vec::new(),
        ephemeral_scopes: Vec::new(),
        retention_max_age_ms: Some(MAX_AGE_MS),
        retention_pressure_grace_ms: Some(GRACE_MS),
    }
}

fn observed(
    available_bytes: Option<u64>,
    ephemeral: Vec<EphemeralEntry>,
) -> BuildCacheObservation {
    BuildCacheObservation {
        schema: BUILD_CACHE_OBSERVATION_SCHEMA,
        node_id: "builder-a".to_string(),
        available_bytes,
        canonical_target: "/state/targets".to_string(),
        canonical_target_state: BuildCachePathState::SafeDirectory,
        reclaim_profile_path: "/state/targets/debug".to_string(),
        reclaim_profile_state: BuildCachePathState::SafeDirectory,
        preserved_profile_states: vec![BuildCachePathState::SafeDirectory],
        observed_noncanonical_cache_paths: Vec::new(),
        observed_ephemeral: ephemeral,
        ephemeral_skipped: Vec::new(),
        unknowns: Vec::new(),
    }
}

fn checkout(path: &str, age_ms: u64) -> EphemeralEntry {
    EphemeralEntry {
        path: path.to_string(),
        age_ms,
        kind: EphemeralKind::EvidenceCheckout,
        workload_terminal: None,
    }
}

fn workspace(path: &str, age_ms: u64, terminal: Option<bool>) -> EphemeralEntry {
    EphemeralEntry {
        path: path.to_string(),
        age_ms,
        kind: EphemeralKind::WorkloadWorkspace,
        workload_terminal: terminal,
    }
}

#[test]
fn pressure_names_terminal_workspaces_and_aged_checkouts_eldest_first() {
    let plan = reconcile_build_cache(
        &desired(),
        &observed(
            Some(10),
            vec![
                checkout("/state/checkouts/cynic-aaa", 1_000),
                workspace("/state/ws/build-abc", 500, Some(true)),
                checkout("/state/checkouts/cynic-bbb", 5_000_000),
            ],
        ),
    );

    assert_eq!(plan.decision, BuildCacheDecision::ReclaimEphemeral);
    assert!(plan.mutates);
    assert_eq!(
        plan.ephemeral_targets,
        vec![
            "/state/checkouts/cynic-bbb".to_string(),
            "/state/ws/build-abc".to_string(),
        ]
    );
}

#[test]
fn live_and_unclassifiable_workspaces_are_never_eligible() {
    let plan = reconcile_build_cache(
        &desired(),
        &observed(
            Some(10),
            vec![
                workspace("/state/ws/live", 9_000_000, Some(false)),
                workspace("/state/ws/unknown", 9_000_000, None),
            ],
        ),
    );

    // No eligible ephemeral: pressure with warm caches falls through to the
    // canonical profile, exactly as before retention existed.
    assert_eq!(plan.decision, BuildCacheDecision::ReclaimCanonicalProfile);
    assert!(plan.ephemeral_targets.is_empty());
}

#[test]
fn young_entries_are_untouched_without_pressure() {
    let plan = reconcile_build_cache(
        &desired(),
        &observed(
            Some(10_000),
            vec![
                checkout("/state/checkouts/cynic-aaa", 1_000),
                workspace("/state/ws/build-abc", 500, Some(true)),
            ],
        ),
    );

    assert_eq!(plan.decision, BuildCacheDecision::Satisfied);
    assert!(!plan.mutates);
    assert!(plan.ephemeral_targets.is_empty());
}

#[test]
fn hard_cap_fires_without_pressure() {
    let plan = reconcile_build_cache(
        &desired(),
        &observed(
            Some(10_000),
            vec![checkout("/state/checkouts/cynic-old", MAX_AGE_MS + 1)],
        ),
    );

    assert_eq!(plan.decision, BuildCacheDecision::ReclaimEphemeral);
    assert_eq!(
        plan.ephemeral_targets,
        vec!["/state/checkouts/cynic-old".to_string()]
    );
}

#[test]
fn young_checkout_is_spared_under_pressure_by_grace() {
    let plan = reconcile_build_cache(
        &desired(),
        &observed(Some(10), vec![checkout("/state/checkouts/cynic-new", 60_000)]),
    );

    // The only ephemeral entry is younger than the pressure grace: no
    // ephemeral decision, canonical profile takes the pressure instead.
    assert_eq!(plan.decision, BuildCacheDecision::ReclaimCanonicalProfile);
    assert!(plan.ephemeral_targets.is_empty());
}

#[test]
fn retention_absent_preserves_historical_behavior() {
    let mut undeclared = desired();
    undeclared.retention_max_age_ms = None;
    undeclared.retention_pressure_grace_ms = None;
    let plan = reconcile_build_cache(
        &undeclared,
        &observed(
            Some(10),
            vec![checkout("/state/checkouts/cynic-old", MAX_AGE_MS + 1_000_000)],
        ),
    );

    assert_eq!(plan.decision, BuildCacheDecision::ReclaimCanonicalProfile);
    assert!(plan.ephemeral_targets.is_empty());
}
