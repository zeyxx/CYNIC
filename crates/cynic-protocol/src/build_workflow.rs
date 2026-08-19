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

use crate::build::{
    admit_build_input, admit_build_runner, compose_build_runner_envelope, validate_build_recipe,
    BuildBootstrapGrant, BuildInputAdmission, BuildRecipe, BuildRunnerAdmission,
    BuildRunnerEnvelope, BuildRunnerObservation,
};
use crate::source_control::SourceSnapshot;
use crate::LeaseVerification;

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
                let source_commit = self
                    .source_snapshot
                    .head_commit
                    .as_deref()
                    .unwrap_or("");
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
            self
                .source_snapshot
                .head_commit
                .clone()
                .unwrap_or_default(),
            self.argv.clone(),
            self.artifact_path.clone(),
            self.apply,
            self.now_unix_ms,
            &self.observation.node_id,
            "build_runner",
        );

        // Step 4: Admit runner
        let runner_admission = admit_build_runner(&envelope);

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
        BuildRecipe, BuildRunnerObservation, BUILD_RECIPE_SCHEMA, BUILD_RUNNER_OBSERVATION_SCHEMA,
    };
    use crate::{LeaseGuard, LeaseVerification, SecurityEvidenceState, SourceSnapshot};

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
        let workflow = BuildWorkflow::new(
            &valid_observation(),
            &valid_lease_verification(),
            &valid_source_snapshot(),
            &valid_recipe(),
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        );
        let outcome = workflow.decide();
        assert!(matches!(outcome, BuildWorkflowOutcome::Planned { .. }));
    }

    #[test]
    fn workflow_returns_execute_when_admission_passes_and_applying() {
        let workflow = BuildWorkflow::new(
            &valid_observation(),
            &valid_lease_verification(),
            &valid_source_snapshot(),
            &valid_recipe(),
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
        let mut dirty_snapshot = valid_source_snapshot();
        dirty_snapshot.unstaged_paths = vec!["src/lib.rs".to_string()];
        let workflow = BuildWorkflow::new(
            &valid_observation(),
            &valid_lease_verification(),
            &dirty_snapshot,
            &valid_recipe(),
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        );
        let outcome = workflow.decide();
        match outcome {
            BuildWorkflowOutcome::Blocked {
                input_blockers, ..
            } => {
                assert!(
                    input_blockers.contains(&"source_worktree_not_clean".to_string())
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn workflow_returns_blocked_when_role_not_observed() {
        let mut bad_observation = valid_observation();
        bad_observation.declared_role = Some("storage".to_string());
        let workflow = BuildWorkflow::new(
            &bad_observation,
            &valid_lease_verification(),
            &valid_source_snapshot(),
            &valid_recipe(),
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        );
        let outcome = workflow.decide();
        assert!(matches!(outcome, BuildWorkflowOutcome::Blocked { .. }));
    }

    #[test]
    fn workflow_returns_blocked_when_lease_rejected() {
        let rejected_lease = LeaseVerification::rejected(LeaseGuard {
            agent_id: "dispatcher-1".to_string(),
            target: "build-runner:builder-1".to_string(),
            lease_id: "lease-1".to_string(),
            fencing_token: 7,
        });
        let workflow = BuildWorkflow::new(
            &valid_observation(),
            &rejected_lease,
            &valid_source_snapshot(),
            &valid_recipe(),
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic".to_string(),
            2,
        );
        let outcome = workflow.decide();
        match outcome {
            BuildWorkflowOutcome::Blocked {
                runner_blockers, ..
            } => {
                assert!(
                    runner_blockers.contains(&"build_runner_lease_unverified".to_string())
                );
            }
            other => panic!("expected Blocked, got {other:?}"),
        }
    }

    #[test]
    fn workflow_returns_blocked_when_argv_contains_shell() {
        let workflow = BuildWorkflow::new(
            &valid_observation(),
            &valid_lease_verification(),
            &valid_source_snapshot(),
            &valid_recipe(),
            vec!["bash".to_string(), "-c".to_string(), "make check".to_string()],
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
}
