//! Build workflow runtime — wires the pure workflow to impure ports.
//!
//! The pure decision lives in `cynic_protocol::build_workflow::BuildWorkflow`.
//! This runtime injects the impure parts:
//!   - observation: SSH or JSON adapter
//!   - lease verification: HTTP adapter
//!   - execution: ArgvRunner
//!
//! The runtime itself is testable by injecting fake adapters.

use cynic_protocol::LeaseGuard;
use cynic_protocol::build::BuildRecipe;
use cynic_protocol::build_workflow::{BuildWorkflow, BuildWorkflowOutcome};
use cynic_protocol::source_control::SourceSnapshot;

use crate::build::{BuildProof, BuildRequest, run_build};
use crate::build_evidence::{BuildLeaseVerificationPort, BuildRunnerObservationPort};
use crate::update_lifecycle::ArgvRunner;

/// Runtime configuration for the build workflow.
pub struct BuildWorkflowConfig<'a> {
    pub observation_adapter: &'a dyn BuildRunnerObservationPort,
    pub lease_adapter: &'a dyn BuildLeaseVerificationPort,
    /// Guard issued by the canonical workload claim/lease path.
    /// None is an explicit blocked state, never a request to synthesize one.
    pub lease_guard: Option<LeaseGuard>,
    pub argv_runner: &'a dyn ArgvRunner,
    pub source_snapshot: SourceSnapshot,
    pub recipe: BuildRecipe,
    pub argv: Vec<String>,
    pub artifact_path: String,
    pub apply: bool,
    pub now_unix_ms: u64,
}

/// Outcome of running the build workflow.
#[derive(Debug)]
pub enum BuildWorkflowResult {
    /// Workflow decided to block. No execution attempted.
    Blocked {
        input_blockers: Vec<String>,
        runner_blockers: Vec<String>,
        recipe_blockers: Vec<String>,
        grant_blockers: Vec<String>,
    },
    /// Workflow decided to plan. No execution attempted.
    Planned { proof: BuildProof },
    /// Workflow decided to execute. Build was run.
    Executed { proof: BuildProof },
}

fn validate_build_lease_guard(guard: &LeaseGuard, node_id: &str) -> Vec<String> {
    let mut blockers = Vec::new();
    if guard.agent_id.trim().is_empty() {
        blockers.push("build_lease_guard_agent_missing".to_string());
    }
    if guard.lease_id.trim().is_empty() {
        blockers.push("build_lease_guard_id_missing".to_string());
    }
    if guard.fencing_token == 0 {
        blockers.push("build_lease_guard_fence_missing".to_string());
    }
    let expected_target = format!("build-runner:{node_id}");
    if guard.target != expected_target {
        blockers.push("build_lease_guard_target_mismatch".to_string());
    }
    blockers
}

/// Run the full build workflow: observe → verify → decide → (optionally) execute.
pub fn run_build_workflow(config: &BuildWorkflowConfig) -> Result<BuildWorkflowResult, String> {
    // Step 1: Observe the build runner
    let observation = config
        .observation_adapter
        .observe()
        .map_err(|e| format!("observation failed: {e}"))?;

    // Step 2: Verify the caller-provided lease guard. Missing authority blocks
    // before the pure workflow can produce an executable decision.
    let Some(guard) = config.lease_guard.as_ref() else {
        return Ok(BuildWorkflowResult::Blocked {
            input_blockers: Vec::new(),
            runner_blockers: Vec::new(),
            recipe_blockers: Vec::new(),
            grant_blockers: vec!["build_lease_guard_missing".to_string()],
        });
    };
    let guard_blockers = validate_build_lease_guard(guard, &observation.node_id);
    if !guard_blockers.is_empty() {
        return Ok(BuildWorkflowResult::Blocked {
            input_blockers: Vec::new(),
            runner_blockers: guard_blockers,
            recipe_blockers: Vec::new(),
            grant_blockers: Vec::new(),
        });
    }
    let lease_verification = config.lease_adapter.verify(guard);

    // Step 3: Run pure workflow decision
    let workflow = BuildWorkflow::new(
        &observation,
        &lease_verification,
        &config.source_snapshot,
        &config.recipe,
        config.argv.clone(),
        config.artifact_path.clone(),
        config.now_unix_ms,
    )
    .with_apply(config.apply);

    let outcome = workflow.decide();

    match outcome {
        BuildWorkflowOutcome::Blocked {
            input_blockers,
            runner_blockers,
            recipe_blockers,
            grant_blockers,
        } => Ok(BuildWorkflowResult::Blocked {
            input_blockers,
            runner_blockers,
            recipe_blockers,
            grant_blockers,
        }),
        BuildWorkflowOutcome::Planned {
            envelope,
            input_admission,
            runner_admission,
        } => {
            let request = BuildRequest {
                input: input_admission,
                runner_envelope: envelope,
                runner_admission,
                source_commit: config
                    .source_snapshot
                    .head_commit
                    .clone()
                    .unwrap_or_default(),
                argv: config.argv.clone(),
                artifact_path: config.artifact_path.clone(),
                apply: false,
            };
            let proof = run_build(&request, config.argv_runner);
            Ok(BuildWorkflowResult::Planned { proof })
        }
        BuildWorkflowOutcome::Execute {
            envelope,
            input_admission,
            runner_admission,
        } => {
            let request = BuildRequest {
                input: input_admission,
                runner_envelope: envelope,
                runner_admission,
                source_commit: config
                    .source_snapshot
                    .head_commit
                    .clone()
                    .unwrap_or_default(),
                argv: config.argv.clone(),
                artifact_path: config.artifact_path.clone(),
                apply: true,
            };
            let proof = run_build(&request, config.argv_runner);
            Ok(BuildWorkflowResult::Executed { proof })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_evidence::{BuildLeaseVerificationPort, BuildRunnerObservationPort};
    use crate::update_lifecycle::MapArgvRunner;
    use cynic_protocol::BuildRunnerObservation;
    use cynic_protocol::build::BUILD_RECIPE_SCHEMA;
    use cynic_protocol::{LeaseGuard, LeaseVerification, SecurityEvidenceState, SourceSnapshot};

    fn valid_observation() -> BuildRunnerObservation {
        BuildRunnerObservation {
            schema: cynic_protocol::build::BUILD_RUNNER_OBSERVATION_SCHEMA.to_string(),
            status: "observed".to_string(),
            node_id: "builder-1".to_string(),
            hostname: "builder-1".to_string(),
            declared_role: Some("build_runner".to_string()),
            identity_match: SecurityEvidenceState::Proven,
            observed_at_unix_ms: 1,
            registry_path: "/etc/cynic/registry.json".to_string(),
            evidence: serde_json::json!({}),
            evidence_refs: vec!["node-proof".to_string()],
        }
    }

    fn valid_source_snapshot() -> SourceSnapshot {
        SourceSnapshot {
            branch: Some("main".into()),
            head_commit: Some("abc".into()),
            upstream: Some("forgejo/main".into()),
            staged_paths: Vec::new(),
            unstaged_paths: Vec::new(),
            untracked_paths: Vec::new(),
            unmerged_paths: Vec::new(),
            validation: SecurityEvidenceState::Proven,
            sovereign_transport_identity: SecurityEvidenceState::Proven,
            sovereign_authentication: SecurityEvidenceState::Proven,
            remote_commit_matches: SecurityEvidenceState::Proven,
            build_input_digest: None,
            evidence_refs: vec!["git_push_readback".into()],
        }
    }

    fn valid_recipe() -> BuildRecipe {
        BuildRecipe {
            schema: BUILD_RECIPE_SCHEMA.to_string(),
            action: "make:check".to_string(),
            target_profile: "debug".to_string(),
            // The action MUST appear in allowed_actions: validate_build_recipe
            // refuses any recipe whose action is not explicitly admitted, which
            // is the point of the allow-list. The previous fixture declared
            // action "make:check" against allowed ["cargo:check"] and was
            // therefore rejected as build_recipe_action_not_in_allowed_actions.
            allowed_actions: vec!["make:check".to_string(), "cargo:check".to_string()],
            target_crates: vec!["cynic-protocol".to_string()],
            excluded_crates: vec![],
            toolchain_override: None,
            env_overrides: vec![],
            rust_min_stack: 536_870_912,
            cargo_incremental: false,
            memory_limit_bytes: None,
            max_parallel_jobs: 1,
            timeout_secs: 600,
            evidence_refs: vec![],
        }
    }

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

    #[test]
    fn workflow_blocks_when_source_dirty() {
        let observation_adapter = FakeObservationAdapter {
            observation: valid_observation(),
        };
        let lease_adapter = FakeLeaseAdapter {
            verification: LeaseVerification::proved(
                LeaseGuard {
                    agent_id: "local".to_string(),
                    target: "build-runner:builder-1".to_string(),
                    lease_id: "lease-1".to_string(),
                    fencing_token: 1,
                },
                None,
            ),
        };
        let argv_runner = MapArgvRunner { responses: vec![] };
        let mut dirty_snapshot = valid_source_snapshot();
        dirty_snapshot.unstaged_paths = vec!["src/lib.rs".to_string()];

        let config = BuildWorkflowConfig {
            observation_adapter: &observation_adapter,
            lease_adapter: &lease_adapter,
            lease_guard: Some(LeaseGuard {
                agent_id: "local".to_string(),
                target: "build-runner:builder-1".to_string(),
                lease_id: "lease-1".to_string(),
                fencing_token: 1,
            }),
            argv_runner: &argv_runner,
            source_snapshot: dirty_snapshot,
            recipe: valid_recipe(),
            argv: vec!["make".to_string(), "check".to_string()],
            artifact_path: "target/release/cynic".to_string(),
            apply: false,
            now_unix_ms: 2,
        };

        let result = run_build_workflow(&config).expect("workflow should not error");
        match result {
            BuildWorkflowResult::Blocked { input_blockers, .. } => {
                assert!(input_blockers.contains(&"source_worktree_not_clean".to_string()));
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn lease_guard_target_must_match_observed_runner() {
        let guard = LeaseGuard {
            agent_id: "agent-a".to_string(),
            target: "build-runner:builder-2".to_string(),
            lease_id: "lease-1".to_string(),
            fencing_token: 1,
        };
        assert_eq!(
            validate_build_lease_guard(&guard, "builder-1"),
            vec!["build_lease_guard_target_mismatch".to_string()]
        );
    }

    #[test]
    fn workflow_planned_when_admission_passes_and_not_applying() {
        let observation_adapter = FakeObservationAdapter {
            observation: valid_observation(),
        };
        let lease_adapter = FakeLeaseAdapter {
            verification: LeaseVerification::proved(
                LeaseGuard {
                    agent_id: "local".to_string(),
                    target: "build-runner:builder-1".to_string(),
                    lease_id: "lease-1".to_string(),
                    fencing_token: 1,
                },
                None,
            ),
        };
        let argv_runner = MapArgvRunner { responses: vec![] };

        let config = BuildWorkflowConfig {
            observation_adapter: &observation_adapter,
            lease_adapter: &lease_adapter,
            lease_guard: Some(LeaseGuard {
                agent_id: "local".to_string(),
                target: "build-runner:builder-1".to_string(),
                lease_id: "lease-1".to_string(),
                fencing_token: 1,
            }),
            argv_runner: &argv_runner,
            source_snapshot: valid_source_snapshot(),
            recipe: valid_recipe(),
            argv: vec!["make".to_string(), "check".to_string()],
            artifact_path: "target/release/cynic".to_string(),
            apply: false,
            now_unix_ms: 2,
        };

        let result = run_build_workflow(&config).expect("workflow should not error");
        match result {
            BuildWorkflowResult::Planned { proof } => {
                assert_eq!(proof.status, "planned");
            }
            other => panic!("expected Planned, got {other:?}"),
        }
    }
}
