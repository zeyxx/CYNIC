use serde::{Deserialize, Serialize};

use crate::{
    LEASE_VERIFICATION_SCHEMA, LeaseGuard, LeaseVerification, SecurityEvidenceState, SourceSnapshot,
};

mod agent_request;
mod agent_workload;
mod workload_grant;

pub use agent_request::{
    BUILD_AGENT_REQUEST_ADMISSION_SCHEMA, BUILD_AGENT_REQUEST_SCHEMA,
    BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA, BUILD_SOURCE_REFERENCE_SCHEMA, BuildAgentRequest,
    BuildAgentRequestAdmission, BuildReleasePublicationIntent, BuildSourceReference,
    admit_build_agent_request, plan_build_agent_request, valid_release_publication_id,
    validate_build_source_reference, validate_release_publication_intent,
};
pub use agent_workload::{
    build_agent_request_from_workload, build_source_reference_from_workload,
    compose_build_agent_workload,
};
pub use workload_grant::{
    BUILD_WORKLOAD_GRANT_MAX_TTL_SECONDS, BUILD_WORKLOAD_GRANT_SCHEMA,
    BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE, BuildWorkloadGrant,
    SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA, SignedBuildWorkloadGrant,
    build_workload_grant_signing_bytes, canonical_build_action_from_argv,
    derive_build_workload_grant, validate_build_workload_grant,
    validate_signed_build_workload_grant,
};

pub const BUILD_INPUT_ADMISSION_SCHEMA: &str = "cynic.build.input_admission.v1";
pub const BUILD_RUNNER_ENVELOPE_SCHEMA: &str = "cynic.build.runner_envelope.v1";
pub const BUILD_RUNNER_ADMISSION_SCHEMA: &str = "cynic.build.runner_admission.v1";
pub const BUILD_RUNNER_OBSERVATION_SCHEMA: &str = "cynic.runtime.node_observation.v1";
pub const BUILD_BOOTSTRAP_GRANT_SCHEMA: &str = "cynic.build.bootstrap_grant.v1";
pub const BUILD_RUNNER_OBSERVATION_MAX_AGE_MS: u64 = 300_000;
pub const BUILD_SCHEDULER_RUNTIME_CAPABILITIES: &[&str] = &[
    "kernel_to_node_workload_delivery_read_back",
    "scheduler_claim_renewal",
    "scheduler_cancellation_delivery",
    "scheduler_crash_recovery",
    "node_owned_build_grant_machine_signer",
    "build_runner_signature_and_live_workload_verifier",
    "scheduler_to_build_runner_execution_transport",
    "build_completion_and_artifact_read_back",
    "node_workspace_observer_wiring_and_proof",
];

pub fn canonical_build_actions(requested_action: &str) -> Result<Vec<String>, String> {
    match requested_action {
        "make:check" => Ok(["make:check", "cargo:check", "cargo:run", "cargo:test"]
            .into_iter()
            .map(str::to_string)
            .collect()),
        "make:build" => Ok(["make:build", "cargo:build"]
            .into_iter()
            .map(str::to_string)
            .collect()),
        "make:build-release" => Ok(["make:build-release", "cargo:build"]
            .into_iter()
            .map(str::to_string)
            .collect()),
        _ => Err(format!(
            "unsupported canonical build action: {requested_action}"
        )),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildInputAdmission {
    pub schema: String,
    pub status: String,
    pub source_commit: Option<String>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildRunnerEnvelope {
    pub schema: String,
    pub node_id: String,
    pub declared_role: String,
    pub role_observation: SecurityEvidenceState,
    pub source_commit: String,
    pub lease: LeaseGuard,
    pub lease_verification: SecurityEvidenceState,
    pub argv: Vec<String>,
    pub artifact_path: String,
    pub action_budget: u64,
    pub mutation_authorized: bool,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeNodeObservation {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub hostname: String,
    pub declared_role: Option<String>,
    pub identity_match: SecurityEvidenceState,
    pub observed_at_unix_ms: u64,
    pub registry_path: String,
    pub evidence: serde_json::Value,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

pub type BuildRunnerObservation = RuntimeNodeObservation;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildRunnerAdmission {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub source_commit: String,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildBootstrapGrant {
    pub schema: String,
    pub status: String,
    pub grant_id: String,
    pub node_id: String,
    pub role: String,
    pub source_commit: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub allowed_actions: Vec<String>,
    pub evidence_refs: Vec<String>,
}

impl BuildBootstrapGrant {
    pub fn validate_claims(
        &self,
        observed_node_id: &str,
        observed_source_commit: &str,
        requested_action: &str,
        now_unix_s: u64,
    ) -> Result<(), Vec<String>> {
        let mut blockers = Vec::new();
        if self.schema != BUILD_BOOTSTRAP_GRANT_SCHEMA {
            blockers.push("build_bootstrap_grant_schema_mismatch".to_string());
        }
        if self.status != "admitted" {
            blockers.push("build_bootstrap_grant_not_admitted".to_string());
        }
        if self.grant_id.trim().is_empty() {
            blockers.push("build_bootstrap_grant_id_missing".to_string());
        }
        if self.node_id != observed_node_id {
            blockers.push("build_bootstrap_grant_node_mismatch".to_string());
        }
        if self.role != "build_runner" {
            blockers.push("build_bootstrap_grant_role_mismatch".to_string());
        }
        if self.source_commit != observed_source_commit {
            blockers.push("build_bootstrap_grant_source_mismatch".to_string());
        }
        if self.issued_at_unix_s == 0
            || self.expires_at_unix_s <= self.issued_at_unix_s
            || now_unix_s >= self.expires_at_unix_s
        {
            blockers.push("build_bootstrap_grant_expired_or_invalid".to_string());
        }
        if !self
            .allowed_actions
            .iter()
            .any(|action| action == requested_action)
        {
            blockers.push("build_bootstrap_action_not_allowed".to_string());
        }
        if self.evidence_refs.is_empty() {
            blockers.push("build_bootstrap_grant_evidence_missing".to_string());
        }
        if blockers.is_empty() {
            Ok(())
        } else {
            Err(blockers)
        }
    }
}

pub fn compose_build_runner_envelope(
    observation: &BuildRunnerObservation,
    lease_verification: &LeaseVerification,
    source_commit: impl Into<String>,
    argv: Vec<String>,
    artifact_path: impl Into<String>,
    mutation_authorized: bool,
    now_unix_ms: u64,
    expected_node_id: &str,
    expected_role: &str,
) -> BuildRunnerEnvelope {
    let observation_is_fresh = now_unix_ms
        .checked_sub(observation.observed_at_unix_ms)
        .is_some_and(|age| age <= BUILD_RUNNER_OBSERVATION_MAX_AGE_MS);
    let role_observation = if observation.schema == BUILD_RUNNER_OBSERVATION_SCHEMA
        && observation.status == "observed"
        && observation.observed_at_unix_ms > 0
        && observation.identity_match == SecurityEvidenceState::Proven
        && observation.node_id == expected_node_id
        && observation.declared_role.as_deref() == Some(expected_role)
        && observation_is_fresh
        && !observation.evidence_refs.is_empty()
    {
        SecurityEvidenceState::Proven
    } else {
        SecurityEvidenceState::Unknown
    };
    let lease_verification_state = if lease_verification.schema == LEASE_VERIFICATION_SCHEMA
        && lease_verification.status == "proved"
        && lease_verification.blockers.is_empty()
    {
        SecurityEvidenceState::Proven
    } else {
        SecurityEvidenceState::Unknown
    };
    let mut evidence_refs = observation.evidence_refs.clone();
    evidence_refs.push(format!(
        "coord-lease:{}:{}",
        lease_verification.guard.lease_id, lease_verification.guard.fencing_token
    ));
    BuildRunnerEnvelope {
        schema: BUILD_RUNNER_ENVELOPE_SCHEMA.to_string(),
        node_id: observation.node_id.clone(),
        declared_role: observation.declared_role.clone().unwrap_or_default(),
        role_observation,
        source_commit: source_commit.into(),
        lease: lease_verification.guard.clone(),
        lease_verification: lease_verification_state,
        argv,
        artifact_path: artifact_path.into(),
        action_budget: 1,
        mutation_authorized,
        evidence_refs,
    }
}

pub fn admit_build_runner(envelope: &BuildRunnerEnvelope) -> BuildRunnerAdmission {
    assess_build_runner(envelope, true)
}

pub fn plan_build_runner(envelope: &BuildRunnerEnvelope) -> BuildRunnerAdmission {
    assess_build_runner(envelope, false)
}

fn assess_build_runner(
    envelope: &BuildRunnerEnvelope,
    require_mutation_authority: bool,
) -> BuildRunnerAdmission {
    let mut blockers = Vec::new();
    if envelope.schema != BUILD_RUNNER_ENVELOPE_SCHEMA {
        blockers.push("build_runner_envelope_schema_mismatch".to_string());
    }
    if envelope.node_id.trim().is_empty() {
        blockers.push("build_runner_node_id_missing".to_string());
    }
    if envelope.declared_role != "build_runner" {
        blockers.push("build_runner_role_not_observed".to_string());
    }
    if envelope.role_observation != SecurityEvidenceState::Proven {
        blockers.push("build_runner_role_observation_unproven".to_string());
    }
    if envelope.source_commit.trim().is_empty() {
        blockers.push("build_runner_source_commit_missing".to_string());
    }
    if envelope.lease.agent_id.trim().is_empty()
        || envelope.lease.lease_id.trim().is_empty()
        || envelope.lease.target.trim().is_empty()
    {
        blockers.push("build_runner_lease_identity_incomplete".to_string());
    }
    if envelope.lease.fencing_token == 0 {
        blockers.push("build_runner_fencing_token_missing".to_string());
    }
    if envelope.lease_verification != SecurityEvidenceState::Proven {
        blockers.push("build_runner_lease_unverified".to_string());
    }
    let expected_target = format!("build-runner:{}", envelope.node_id);
    if envelope.lease.target != expected_target {
        blockers.push("build_runner_lease_target_mismatch".to_string());
    }
    if envelope.argv.is_empty() || envelope.argv.iter().any(|arg| arg.trim().is_empty()) {
        blockers.push("build_runner_argv_incomplete".to_string());
    }
    if envelope
        .argv
        .first()
        .is_some_and(|command| matches!(command.as_str(), "sh" | "bash" | "dash" | "zsh" | "fish"))
        || envelope
            .argv
            .iter()
            .any(|arg| arg == "-c" || arg == "--command")
    {
        blockers.push("build_runner_shell_composition_forbidden".to_string());
    }
    if envelope.artifact_path.trim().is_empty() {
        blockers.push("build_runner_artifact_path_missing".to_string());
    }
    if envelope.action_budget != 1 {
        blockers.push("build_runner_action_budget_must_equal_one".to_string());
    }
    if require_mutation_authority && !envelope.mutation_authorized {
        blockers.push("build_runner_mutation_not_authorized".to_string());
    }
    if envelope.evidence_refs.is_empty() {
        blockers.push("build_runner_evidence_missing".to_string());
    }
    BuildRunnerAdmission {
        schema: BUILD_RUNNER_ADMISSION_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "admitted".to_string()
        } else {
            "blocked".to_string()
        },
        node_id: envelope.node_id.clone(),
        source_commit: envelope.source_commit.clone(),
        blockers,
        evidence_refs: envelope.evidence_refs.clone(),
    }
}

pub fn admit_build_input(snapshot: &SourceSnapshot) -> BuildInputAdmission {
    let mut blockers = Vec::new();
    if snapshot.head_commit.as_deref().is_none_or(str::is_empty) {
        blockers.push("source_commit_missing".to_string());
    }
    if !snapshot.staged_paths.is_empty()
        || !snapshot.unstaged_paths.is_empty()
        || !snapshot.untracked_paths.is_empty()
    {
        blockers.push("source_worktree_not_clean".to_string());
    }
    if !snapshot.unmerged_paths.is_empty() {
        blockers.push("source_unmerged_paths_present".to_string());
    }
    if snapshot.validation != SecurityEvidenceState::Proven {
        blockers.push("source_validation_unproven".to_string());
    }
    if snapshot.sovereign_transport_identity != SecurityEvidenceState::Proven {
        blockers.push("source_transport_identity_unproven".to_string());
    }
    if snapshot.sovereign_authentication != SecurityEvidenceState::Proven {
        blockers.push("source_authentication_unproven".to_string());
    }
    if snapshot.remote_commit_matches != SecurityEvidenceState::Proven {
        blockers.push("source_remote_commit_match_unproven".to_string());
    }
    if snapshot.evidence_refs.is_empty() {
        blockers.push("source_evidence_missing".to_string());
    }
    BuildInputAdmission {
        schema: BUILD_INPUT_ADMISSION_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "admitted"
        } else {
            "blocked"
        }
        .to_string(),
        source_commit: snapshot.head_commit.clone(),
        blockers,
        evidence_refs: snapshot.evidence_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_check_expands_to_the_exact_nested_gate_actions() {
        assert_eq!(
            canonical_build_actions("make:check").expect("known gate"),
            vec!["make:check", "cargo:check", "cargo:run", "cargo:test"]
        );
        assert!(canonical_build_actions("bash -c make check").is_err());
    }

    #[test]
    fn release_build_expands_to_its_exact_make_and_cargo_actions() {
        assert_eq!(
            canonical_build_actions("make:build-release").expect("release build"),
            vec!["make:build-release", "cargo:build"]
        );
    }

    #[test]
    fn build_requires_post_push_source_proof() {
        let snapshot = SourceSnapshot {
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
            remote_commit_matches: SecurityEvidenceState::Unknown,
            build_input_digest: None,
            evidence_refs: vec!["git_push_readback".into()],
        };
        let admission = admit_build_input(&snapshot);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"source_remote_commit_match_unproven".into())
        );
    }

    #[test]
    fn build_runner_admission_binds_role_lease_budget_and_argv() {
        let envelope = BuildRunnerEnvelope {
            schema: BUILD_RUNNER_ENVELOPE_SCHEMA.to_string(),
            node_id: "builder-1".to_string(),
            declared_role: "build_runner".to_string(),
            role_observation: SecurityEvidenceState::Proven,
            source_commit: "abc".to_string(),
            lease: LeaseGuard {
                agent_id: "dispatcher-1".to_string(),
                target: "build-runner:builder-1".to_string(),
                lease_id: "lease-1".to_string(),
                fencing_token: 7,
            },
            lease_verification: SecurityEvidenceState::Proven,
            argv: vec!["make".to_string(), "check".to_string()],
            artifact_path: "target/release/cynic".to_string(),
            action_budget: 1,
            mutation_authorized: true,
            evidence_refs: vec!["node-role-proof".to_string(), "lease-proof".to_string()],
        };
        assert_eq!(admit_build_runner(&envelope).status, "admitted");

        let mut amplified = envelope;
        amplified.action_budget = 2;
        let admission = admit_build_runner(&amplified);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"build_runner_action_budget_must_equal_one".to_string())
        );
    }

    #[test]
    fn envelope_composition_does_not_promote_rejected_lease() {
        let observation = BuildRunnerObservation {
            schema: BUILD_RUNNER_OBSERVATION_SCHEMA.to_string(),
            status: "observed".to_string(),
            node_id: "builder-1".to_string(),
            hostname: "builder-1".to_string(),
            declared_role: Some("build_runner".to_string()),
            identity_match: SecurityEvidenceState::Proven,
            observed_at_unix_ms: 1,
            registry_path: "/etc/cynic/registry.json".to_string(),
            evidence: serde_json::json!({"hostname": "builder-1"}),
            evidence_refs: vec!["node-proof".to_string()],
        };
        let guard = LeaseGuard {
            agent_id: "dispatcher-1".to_string(),
            target: "build-runner:builder-1".to_string(),
            lease_id: "lease-1".to_string(),
            fencing_token: 7,
        };
        let verification = LeaseVerification::rejected(guard);
        let envelope = compose_build_runner_envelope(
            &observation,
            &verification,
            "abc",
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic",
            true,
            2,
            "builder-1",
            "build_runner",
        );
        let admission = admit_build_runner(&envelope);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"build_runner_lease_unverified".to_string())
        );
    }

    #[test]
    fn stale_node_observation_cannot_authorize_build() {
        let observation = BuildRunnerObservation {
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
        };
        let guard = LeaseGuard {
            agent_id: "dispatcher-1".to_string(),
            target: "build-runner:builder-1".to_string(),
            lease_id: "lease-1".to_string(),
            fencing_token: 7,
        };
        let verification = LeaseVerification::proved(guard, None);
        let envelope = compose_build_runner_envelope(
            &observation,
            &verification,
            "abc",
            vec!["make".to_string(), "check".to_string()],
            "target/release/cynic",
            true,
            BUILD_RUNNER_OBSERVATION_MAX_AGE_MS + 2,
            "builder-1",
            "build_runner",
        );
        let admission = admit_build_runner(&envelope);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"build_runner_role_observation_unproven".to_string())
        );
    }

    #[test]
    fn bootstrap_grant_is_bound_to_node_source_expiry_and_action() {
        let grant = BuildBootstrapGrant {
            schema: BUILD_BOOTSTRAP_GRANT_SCHEMA.to_string(),
            status: "admitted".to_string(),
            grant_id: "bootstrap-1".to_string(),
            node_id: "cynic-builder".to_string(),
            role: "build_runner".to_string(),
            source_commit: "abc".to_string(),
            issued_at_unix_s: 100,
            expires_at_unix_s: 200,
            allowed_actions: vec!["cargo:check".to_string()],
            evidence_refs: vec!["machine-enrollment:bootstrap-1".to_string()],
        };

        assert!(
            grant
                .validate_claims("cynic-builder", "abc", "cargo:check", 150)
                .is_ok()
        );
        assert!(
            grant
                .validate_claims("cortex", "abc", "cargo:check", 150)
                .expect_err("wrong node")
                .contains(&"build_bootstrap_grant_node_mismatch".to_string())
        );
    }
}
