//! End-to-end build workflow integration test.
//!
//! Proves the full flow without real SSH/HTTP/cargo by injecting fake adapters.
//! This is the canonical proof that the build capability works end-to-end.

use cynic_host::build_evidence::{BuildLeaseVerificationPort, BuildRunnerObservationPort};
use cynic_host::build_workflow::{BuildWorkflowConfig, BuildWorkflowResult, run_build_workflow};
use cynic_host::update_lifecycle::{ArgvOutput, ArgvRunner};
use cynic_protocol::build::{BuildRecipe, BuildRunnerObservation, build_recipe};
use cynic_protocol::{LeaseGuard, LeaseVerification, SecurityEvidenceState, SourceSnapshot};

// ── Fake adapters ─────────────────────────────────────────────

struct FakeObservationAdapter {
    observation: BuildRunnerObservation,
}

impl BuildRunnerObservationPort for FakeObservationAdapter {
    fn observe(&self) -> Result<BuildRunnerObservation, String> {
        Ok(self.observation.clone())
    }
}

struct FakeLeaseAdapter {
    verification: LeaseVerification,
}

impl BuildLeaseVerificationPort for FakeLeaseAdapter {
    fn verify(&self, _guard: &LeaseGuard) -> LeaseVerification {
        self.verification.clone()
    }
}

struct FakeArgvRunner {
    outputs: Vec<ArgvOutput>,
}

impl ArgvRunner for FakeArgvRunner {
    fn run(&self, argv: &[&str]) -> ArgvOutput {
        // Simulate cargo check success
        ArgvOutput {
            exit_code: Some(0),
            stdout: format!("{}: success", argv.join(" ")),
            stderr: String::new(),
            spawn_error: None,
        }
    }
}

// ── Test fixtures ─────────────────────────────────────────────

fn valid_observation() -> BuildRunnerObservation {
    BuildRunnerObservation {
        schema: "cynic.runtime.node_observation.v1".to_string(),
        status: "converged".to_string(),
        node_id: "test-runner".to_string(),
        hostname: "test-host".to_string(),
        declared_role: Some("build_runner".to_string()),
        identity_match: SecurityEvidenceState::Proven,
        observed_at_unix_ms: 1_700_000_000_000,
        registry_path: "/etc/cynic/registry.json".to_string(),
        evidence: serde_json::json!({}),
        evidence_refs: vec!["test-evidence".to_string()],
    }
}

fn valid_lease_verification() -> LeaseVerification {
    LeaseVerification::proved(
        LeaseGuard {
            agent_id: "test-agent".to_string(),
            target: "build-runner:test-runner".to_string(),
            lease_id: "lease-test".to_string(),
            fencing_token: 1,
        },
        Some("2026-12-31T23:59:59Z".to_string()),
    )
}

fn valid_source_snapshot() -> SourceSnapshot {
    SourceSnapshot {
        branch: Some("main".into()),
        head_commit: Some("abc123def456".to_string()),
        upstream: Some("forgejo/main".into()),
        staged_paths: vec![],
        unstaged_paths: vec![],
        untracked_paths: vec![],
        unmerged_paths: vec![],
        validation: SecurityEvidenceState::Proven,
        sovereign_transport_identity: SecurityEvidenceState::Proven,
        sovereign_authentication: SecurityEvidenceState::Proven,
        remote_commit_matches: SecurityEvidenceState::Proven,
        build_input_digest: None,
        evidence_refs: vec!["test-input".into()],
    }
}

fn valid_recipe() -> BuildRecipe {
    // Derive from the canonical constructor instead of restating every field:
    // the recipe grew allowed_actions, rust_min_stack, cargo_incremental,
    // memory_limit_bytes and target_profile, and a literal fixture goes stale on
    // each addition. Only the fields this test actually exercises are overridden.
    let mut recipe = build_recipe("make:check").expect("make:check is a canonical recipe");
    recipe.target_crates = vec!["cynic-protocol".to_string()];
    recipe.evidence_refs = vec![];
    recipe
}

fn base_config<'a>(
    observation_adapter: &'a FakeObservationAdapter,
    lease_adapter: &'a FakeLeaseAdapter,
    argv_runner: &'a FakeArgvRunner,
) -> BuildWorkflowConfig<'a> {
    BuildWorkflowConfig {
        observation_adapter,
        lease_adapter,
        argv_runner,
        source_snapshot: valid_source_snapshot(),
        recipe: valid_recipe(),
        argv: vec!["make".to_string(), "check".to_string()],
        artifact_path: "target/release/cynic".to_string(),
        apply: false,
        now_unix_ms: 1_700_000_000_000,
    }
}

// ── Tests ─────────────────────────────────────────────────────

#[test]
fn e2e_workflow_planned_when_all_admitted() {
    let observation_adapter = FakeObservationAdapter {
        observation: valid_observation(),
    };
    let lease_adapter = FakeLeaseAdapter {
        verification: valid_lease_verification(),
    };
    let argv_runner = FakeArgvRunner { outputs: vec![] };
    let config = base_config(&observation_adapter, &lease_adapter, &argv_runner);

    let result = run_build_workflow(&config);

    assert!(
        result.is_ok(),
        "workflow should succeed: {:?}",
        result.err()
    );
    let result = result.unwrap();
    match result {
        BuildWorkflowResult::Planned { proof } => {
            assert_eq!(proof.status, "planned");
            assert_eq!(proof.source_commit, "abc123def456");
        }
        other => panic!("expected Planned, got {:?}", other),
    }
}

#[test]
fn e2e_workflow_blocked_when_lease_rejected() {
    let observation_adapter = FakeObservationAdapter {
        observation: valid_observation(),
    };
    let lease_adapter = FakeLeaseAdapter {
        verification: LeaseVerification::rejected(LeaseGuard {
            agent_id: "test-agent".to_string(),
            target: "build-runner:test-runner".to_string(),
            lease_id: "lease-test".to_string(),
            fencing_token: 1,
        }),
    };
    let argv_runner = FakeArgvRunner { outputs: vec![] };
    let config = base_config(&observation_adapter, &lease_adapter, &argv_runner);

    let result = run_build_workflow(&config);

    assert!(
        result.is_ok(),
        "workflow should succeed: {:?}",
        result.err()
    );
    let result = result.unwrap();
    match result {
        BuildWorkflowResult::Blocked { grant_blockers, .. } => {
            assert!(!grant_blockers.is_empty(), "should have grant blockers");
        }
        other => panic!("expected Blocked, got {:?}", other),
    }
}

#[test]
fn e2e_workflow_blocked_when_observation_stale() {
    let mut observation = valid_observation();
    observation.observed_at_unix_ms = 0; // stale
    let observation_adapter = FakeObservationAdapter { observation };
    let lease_adapter = FakeLeaseAdapter {
        verification: valid_lease_verification(),
    };
    let argv_runner = FakeArgvRunner { outputs: vec![] };
    let config = base_config(&observation_adapter, &lease_adapter, &argv_runner);

    let result = run_build_workflow(&config);

    assert!(
        result.is_ok(),
        "workflow should succeed: {:?}",
        result.err()
    );
    let result = result.unwrap();
    match result {
        BuildWorkflowResult::Blocked {
            runner_blockers, ..
        } => {
            assert!(!runner_blockers.is_empty(), "should have runner blockers");
        }
        other => panic!("expected Blocked, got {:?}", other),
    }
}

#[test]
fn e2e_workflow_executed_when_apply_true() {
    let observation_adapter = FakeObservationAdapter {
        observation: valid_observation(),
    };
    let lease_adapter = FakeLeaseAdapter {
        verification: valid_lease_verification(),
    };
    let argv_runner = FakeArgvRunner { outputs: vec![] };
    let mut config = base_config(&observation_adapter, &lease_adapter, &argv_runner);
    config.apply = true;

    let result = run_build_workflow(&config);

    assert!(
        result.is_ok(),
        "workflow should succeed: {:?}",
        result.err()
    );
    let result = result.unwrap();
    match result {
        BuildWorkflowResult::Executed { proof } => {
            assert_eq!(proof.status, "executed");
            assert_eq!(proof.source_commit, "abc123def456");
        }
        other => panic!("expected Executed, got {:?}", other),
    }
}

#[test]
fn e2e_workflow_blocked_when_source_unproven() {
    let observation_adapter = FakeObservationAdapter {
        observation: valid_observation(),
    };
    let lease_adapter = FakeLeaseAdapter {
        verification: valid_lease_verification(),
    };
    let argv_runner = FakeArgvRunner { outputs: vec![] };
    let mut config = base_config(&observation_adapter, &lease_adapter, &argv_runner);
    // SecurityEvidenceState has no `Unproven` variant: "unproven" is a policy
    // outcome, not an evidence state. admit_build_input blocks on `!= Proven`
    // (build/mod.rs:365), so `Declared` — asserted but not proven — is the state
    // that exercises this path.
    config.source_snapshot.validation = SecurityEvidenceState::Declared;

    let result = run_build_workflow(&config);

    assert!(
        result.is_ok(),
        "workflow should succeed: {:?}",
        result.err()
    );
    let result = result.unwrap();
    match result {
        BuildWorkflowResult::Blocked { input_blockers, .. } => {
            assert!(
                input_blockers.contains(&"source_validation_unproven".to_string()),
                "expected the validation blocker, got {input_blockers:?}"
            );
        }
        other => panic!("expected Blocked, got {:?}", other),
    }
}
