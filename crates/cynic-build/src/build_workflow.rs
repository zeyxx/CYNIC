//! Build workflow — pure, sequential, testable without IO.
//!
//! Composes the existing admission primitives into a single decision pipeline:
//!   1. observe node → observation
//!   2. verify lease → verification
//!   3. compose envelope (observation + lease)
//!   4. admit runner (envelope)
//!   5. admit input (source snapshot)
//!   6. decide: blocked | planned | execute
//!
//! The workflow is pure: no process spawn, no network, no filesystem.
//! The impure parts (observation, lease verification, execution) are injected
//! via ports so the workflow itself is unit-testable.

use cynic_protocol::LeaseVerification;
use crate::build::{
    BuildBootstrapGrant, BuildInputAdmission, BuildRecipe, BuildRunnerAdmission,
    BuildRunnerEnvelope, BuildRunnerObservation, admit_build_input, admit_build_runner,
    compose_build_runner_envelope, plan_build_runner, validate_build_recipe,
};
use cynic_protocol::source_control::SourceSnapshot;

/// Pure build workflow decision — no IO, no process spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildWorkflow<'a> {
    pub observation: &'a BuildRunnerObservation,
    pub lease_verification: &'a LeaseVerification,
    pub source_snapshot: &'a SourceSnapshot,
    pub recipe: &'a BuildRecipe,
    pub grant: Option<&'a BuildBootstrapGrant>,
    pub argv: Vec<String>,
    pub artifact_path: String,
    pub apply: bool,
    pub now_unix_ms: u64,
}

/// Workflow outcome — what the pure decision layer recommends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildWorkflowOutcome {
    /// Blocked: cannot proceed. Contains all blockers.
    Blocked {
        input_blockers: Vec<String>,
        runner_blockers: Vec<String>,
        recipe_blockers: Vec<String>,
        grant_blockers: Vec<String>,
    },
    /// Planned: admission passed but apply=false. Safe to execute.
    Planned {
        envelope: BuildRunnerEnvelope,
        input_admission: BuildInputAdmission,
        runner_admission: BuildRunnerAdmission,
    },
    /// Execute: admission passed and apply=true. Ready to run.
    Execute {
        envelope: BuildRunnerEnvelope,
        input_admission: BuildInputAdmission,
        runner_admission: BuildRunnerAdmission,
    },
}

impl<'a> BuildWorkflow<'a> {
    pub fn new(
        observation: &'a BuildRunnerObservation,
        lease_verification: &'a LeaseVerification,
        source_snapshot: &'a SourceSnapshot,
        recipe: &'a BuildRecipe,
        argv: Vec<String>,
        artifact_path: String,
        now_unix_ms: u64,
    ) -> Self {
        Self {
            observation,
            lease_verification,
            source_snapshot,
            recipe,
            grant: None,
            argv,
            artifact_path,
            apply: false,
            now_unix_ms,
        }
    }

    pub fn with_grant(mut self, grant: &'a BuildBootstrapGrant) -> Self {
        self.grant = Some(grant);
        self
    }

    pub fn with_apply(mut self, apply: bool) -> Self {
        self.apply = apply;
        self
    }

    /// Run the pure workflow decision.
    pub fn decide(&self) -> BuildWorkflowOutcome {
        // Step 1: Validate recipe
        let recipe_blockers = match validate_build_recipe(self.recipe) {
            Ok(()) => vec![],
            Err(blockers) => vec![blockers],
        };

        // Step 2: Validate grant (if present)
        let grant_blockers = match self.grant {
            Some(grant) => {
                let source_commit = self.source_snapshot.head_commit.as_deref().unwrap_or("");
                let now_unix_s = self.now_unix_ms / 1000;
                match grant.validate_claims(
                    &self.observation.node_id,
                    source_commit,
                    self.recipe.action.as_str(),
                    now_unix_s,
                ) {
                    Ok(()) => vec![],
                    Err(blockers) => blockers,
                }
            }
            None => vec![],
        };

        // Step 3: Compose envelope
        let envelope = compose_build_runner_envelope(
            self.observation,
            self.lease_verification,
            self.source_snapshot.head_commit.clone().unwrap_or_default(),
            self.argv.clone(),
            self.artifact_path.clone(),
            self.apply,
            self.now_unix_ms,
            &self.observation.node_id,
            "build_runner",
        );

        // Step 4: Admit runner.
        //
        // Planning is a read-only reconciliation and must not require mutation
        // authority; only an applying workflow does. Using the mutation-gated
        // `admit_build_runner` for both paths made `Planned` unreachable
        // (`build_runner_mutation_not_authorized` always fired when
        // `apply == false`, since the envelope derives `mutation_authorized`
        // from `apply`). Contract: desired state may be planned without
        // mutation authority; only bounded action requires it.
        let runner_admission = if self.apply {
            admit_build_runner(&envelope)
        } else {
            plan_build_runner(&envelope)
        };

        // Step 5: Admit input
        let input_admission = admit_build_input(self.source_snapshot);

        // Step 6: Decide
        let input_blockers = if input_admission.status == "admitted" {
            vec![]
        } else {
            input_admission.blockers.clone()
        };

        let runner_blockers = if runner_admission.status == "admitted" {
            vec![]
        } else {
            runner_admission.blockers.clone()
        };

        if !recipe_blockers.is_empty()
            || !grant_blockers.is_empty()
            || !input_blockers.is_empty()
            || !runner_blockers.is_empty()
        {
            return BuildWorkflowOutcome::Blocked {
                input_blockers,
                runner_blockers,
                recipe_blockers,
                grant_blockers,
            };
        }

        if self.apply {
            BuildWorkflowOutcome::Execute {
                envelope,
                input_admission,
                runner_admission,
            }
        } else {
            BuildWorkflowOutcome::Planned {
                envelope,
                input_admission,
                runner_admission,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::{
        BUILD_RECIPE_SCHEMA, BUILD_RUNNER_OBSERVATION_SCHEMA, BuildRecipe, BuildRunnerObservation,
    };
    use cynic_protocol::{LeaseGuard, LeaseVerification, SecurityEvidenceState, SourceSnapshot};

    fn valid_observation() -> BuildRunnerObservation {
        BuildRunnerObservation {
            schema: BUILD_RUNNER_OBSERVATION_SCHEMA.to_string(),
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

    fn valid_lease_verification() -> LeaseVerification {
        LeaseVerification::proved(
            LeaseGuard {
                agent_id: "dispatcher-1".to_string(),
                target: "build-runner:builder-1".to_string(),
                lease_id: "lease-1".to_string(),
                fencing_token: 7,
            },
            None,
        )
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
            allowed_actions: vec![
                "make:check".to_string(),
                "cargo:check".to_string(),
                "cargo:run".to_string(),
                "cargo:test".to_string(),
            ],
            rust_min_stack: 536_870_912,
            cargo_incremental: false,
            memory_limit_bytes: Some(4_294_967_296),
            target_crates: vec!["cynic-protocol".to_string()],
            excluded_crates: vec![],
            toolchain_override: None,
            env_overrides: vec![],
            max_parallel_jobs: 1,
            timeout_secs: 600,
            evidence_refs: vec![],
        }
    }

    #[test]
    fn workflow_returns_planned_when_admission_passes_and_not_applying() {
        let observation = valid_observation();
        let lease = valid_lease_verification();
        let snapshot = valid_source_snapshot();
        let recipe = valid_recipe();
        let workflow = BuildWorkflow::new(
            &observation,
            &lease,
            &snapshot,
            &recipe,
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        );
        let outcome = workflow.decide();
        assert!(matches!(outcome, BuildWorkflowOutcome::Planned { .. }));
    }

    #[test]
    fn workflow_returns_execute_when_admission_passes_and_applying() {
        let observation = valid_observation();
        let lease = valid_lease_verification();
        let snapshot = valid_source_snapshot();
        let recipe = valid_recipe();
        let workflow = BuildWorkflow::new(
            &observation,
            &lease,
            &snapshot,
            &recipe,
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        )
        .with_apply(true);
        let outcome = workflow.decide();
        assert!(matches!(outcome, BuildWorkflowOutcome::Execute { .. }));
    }

    #[test]
    fn workflow_returns_blocked_when_source_dirty() {
        let observation = valid_observation();
        let lease = valid_lease_verification();
        let recipe = valid_recipe();
        let mut dirty_snapshot = valid_source_snapshot();
        dirty_snapshot.unstaged_paths = vec!["src/lib.rs".to_string()];
        let workflow = BuildWorkflow::new(
            &observation,
            &lease,
            &dirty_snapshot,
            &recipe,
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        );
        let outcome = workflow.decide();
        match outcome {
            BuildWorkflowOutcome::Blocked { input_blockers, .. } => {
                assert!(input_blockers.contains(&"source_worktree_not_clean".to_string()));
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn workflow_returns_blocked_when_role_not_observed() {
        let lease = valid_lease_verification();
        let snapshot = valid_source_snapshot();
        let recipe = valid_recipe();
        let mut bad_observation = valid_observation();
        bad_observation.declared_role = Some("storage".to_string());
        let workflow = BuildWorkflow::new(
            &bad_observation,
            &lease,
            &snapshot,
            &recipe,
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        );
        let outcome = workflow.decide();
        assert!(matches!(outcome, BuildWorkflowOutcome::Blocked { .. }));
    }

    #[test]
    fn workflow_returns_blocked_when_lease_rejected() {
        let observation = valid_observation();
        let snapshot = valid_source_snapshot();
        let recipe = valid_recipe();
        let rejected_lease = LeaseVerification::rejected(LeaseGuard {
            agent_id: "dispatcher-1".to_string(),
            target: "build-runner:builder-1".to_string(),
            lease_id: "lease-1".to_string(),
            fencing_token: 7,
        });
        let workflow = BuildWorkflow::new(
            &observation,
            &rejected_lease,
            &snapshot,
            &recipe,
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        );
        let outcome = workflow.decide();
        match outcome {
            BuildWorkflowOutcome::Blocked {
                runner_blockers, ..
            } => {
                assert!(runner_blockers.contains(&"build_runner_lease_unverified".to_string()));
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn workflow_returns_blocked_when_argv_contains_shell() {
        let observation = valid_observation();
        let lease = valid_lease_verification();
        let snapshot = valid_source_snapshot();
        let recipe = valid_recipe();
        let workflow = BuildWorkflow::new(
            &observation,
            &lease,
            &snapshot,
            &recipe,
            vec![
                "bash".to_string(),
                "-c".to_string(),
                "make check".to_string(),
            ],
            "target/release/cynic".to_string(),
            2,
        );
        let outcome = workflow.decide();
        match outcome {
            BuildWorkflowOutcome::Blocked {
                runner_blockers, ..
            } => {
                assert!(
                    runner_blockers
                        .contains(&"build_runner_shell_composition_forbidden".to_string())
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    /// Regression proof for the plan-vs-apply mutation authority contract.
    ///
    /// `compose_build_runner_envelope` derives `mutation_authorized` from
    /// `apply`. If `decide()` used the mutation-gated `admit_build_runner` on
    /// both paths, planning could never succeed: the envelope would always
    /// carry `mutation_authorized == false` and raise
    /// `build_runner_mutation_not_authorized`. Read-only reconciliation must
    /// never require mutation authority.
    #[test]
    fn planning_does_not_require_mutation_authority() {
        let observation = valid_observation();
        let lease = valid_lease_verification();
        let snapshot = valid_source_snapshot();
        let recipe = valid_recipe();
        let plan = BuildWorkflow::new(
            &observation,
            &lease,
            &snapshot,
            &recipe,
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        );

        match plan.decide() {
            BuildWorkflowOutcome::Planned {
                runner_admission,
                envelope,
                ..
            } => {
                // The envelope truthfully reports no mutation authority ...
                assert!(!envelope.mutation_authorized);
                // ... yet the planning admission is clean.
                assert_eq!(runner_admission.status, "admitted");
                assert!(
                    !runner_admission
                        .blockers
                        .contains(&"build_runner_mutation_not_authorized".to_string()),
                    "planning must not be gated on mutation authority"
                );
            }
            other => panic!("expected Planned, got {other:?}"),
        }

        // Applying with authority stays admitted and carries the authority.
        let apply = BuildWorkflow::new(
            &observation,
            &lease,
            &snapshot,
            &recipe,
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        )
        .with_apply(true);

        match apply.decide() {
            BuildWorkflowOutcome::Execute {
                runner_admission,
                envelope,
                ..
            } => {
                assert!(envelope.mutation_authorized);
                assert_eq!(runner_admission.status, "admitted");
            }
            other => panic!("expected Execute, got {other:?}"),
        }
    }
}
