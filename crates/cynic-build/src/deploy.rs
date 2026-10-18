//! Pure external deploy governance for build artifacts.
//!
//! WHY: build artifacts (from cynic-build or external projects) must be
//! deployed to target hosts with admission control, rollback capability, and
//! health verification. This module defines the `DeployPort` trait and pure
//! types that abstract deployment without coupling to SSH, HTTP, or process
//! spawn.
//! HOW: trait + pure data types + pure admission validators + pure proof
//! builder. I/O lives in adapters (cynic-node, cynic-host).
//! NEGATIVE BOUNDARY: no std::process::Command, no SSH, no HTTP, no filesystem
//! I/O. Host identity comes from infra/registry.json (observed, not fabricated).

use serde::{Deserialize, Serialize};

use crate::ports::BuildCommandOutput;

// ── Schemas ───────────────────────────────────────────────────────────────────

pub const DEPLOY_SCHEMA: &str = "cynic.deploy.external.v1";
pub const DEPLOY_PROOF_SCHEMA: &str = "cynic.deploy.external_proof.v1";

// ── Mutation policy ───────────────────────────────────────────────────────────

/// Domain-level mutation policy for deploy operations.
///
/// Controls whether a deploy request mutates the target host. Mirrors the
/// fail-closed pattern from `SecretMutationPolicy`: unknown values deny.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployMutationPolicy {
    /// No mutation of the target host is permitted.
    ReadOnly,
    /// Mutation requires an explicit `--apply` gesture.
    ExplicitApplyOnly,
    /// Mutation requires an admitted dispatch or explicit apply.
    AdmittedDispatchOrExplicitApplyOnly,
    /// Policy string present but unrecognised. Fail closed: treated as ReadOnly.
    Unrecognised,
}

impl DeployMutationPolicy {
    /// Parse the declarative spelling. Unknown values fail closed to
    /// `Unrecognised`, which denies mutation — never silently permits.
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "read_only" => Self::ReadOnly,
            "explicit_apply_only" => Self::ExplicitApplyOnly,
            "admitted_dispatch_or_explicit_apply_only" => Self::AdmittedDispatchOrExplicitApplyOnly,
            _ => Self::Unrecognised,
        }
    }

    /// Whether this policy permits a deploy that mutates the target host.
    pub fn permits_mutation(self) -> bool {
        matches!(
            self,
            Self::ExplicitApplyOnly | Self::AdmittedDispatchOrExplicitApplyOnly
        )
    }
}

// ── Core types ────────────────────────────────────────────────────────────────

/// Request to deploy a build artifact to a target host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployRequest {
    /// Stable deployment target slug (e.g. "cynic-builder", "staging-node-1").
    pub target_host_id: String,
    /// Artifact locator (e.g. content-addressed digest or path).
    pub artifact_ref: String,
    /// Deployment kind.
    pub kind: DeployKind,
    /// Mutation policy observed from the target host declaration.
    pub mutation_policy: DeployMutationPolicy,
    /// Whether the caller has explicitly authorised mutation (--apply).
    pub apply: bool,
    /// Optional health-check command argv.
    pub health_check_argv: Option<Vec<String>>,
    /// Optional rollback artifact ref (previous version).
    pub previous_artifact_ref: Option<String>,
}

/// Deploy vs rollback vs health-check distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployKind {
    Deploy,
    Rollback,
    HealthCheck,
}

/// Result of a single deploy operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployResult {
    /// Which operation was performed.
    pub kind: DeployKind,
    /// Target host identifier.
    pub target_host_id: String,
    /// Artifact ref that was deployed (or rolled back to).
    pub artifact_ref: String,
    /// Whether the operation succeeded.
    pub success: bool,
    /// Output from the deploy command.
    pub output: Option<DeployOutput>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Captured output from a deploy command execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl DeployOutput {
    pub fn from_command_output(output: &BuildCommandOutput) -> Self {
        Self {
            stdout: output.stdout.clone(),
            stderr: output.stderr.clone(),
            exit_code: output.exit_code,
        }
    }

    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }
}

/// Proof emitted after a deploy operation completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployProof {
    pub schema: &'static str,
    pub target_host_id: String,
    pub kind: DeployKind,
    pub artifact_ref: String,
    pub success: bool,
    pub rolled_back: bool,
    pub health_check_passed: bool,
    pub mutation_policy: DeployMutationPolicy,
    pub admitted: bool,
    pub blockers: Vec<String>,
    pub output: Option<DeployOutput>,
}

// ── Admission (pure) ──────────────────────────────────────────────────────────

/// Admission report for an external deploy operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployAdmission {
    pub admitted: bool,
    pub blockers: Vec<String>,
}

/// Validate that a deploy request is eligible for execution.
///
/// Pure: no I/O. Checks:
/// - target_host_id is non-empty
/// - artifact_ref is non-empty
/// - mutation_policy permits mutation (fail-closed)
/// - apply flag is set when policy requires it
pub fn admit_external_deploy(
    target_host_id: &str,
    artifact_ref: &str,
    mutation_policy: DeployMutationPolicy,
    apply: bool,
) -> DeployAdmission {
    let mut blockers = Vec::new();

    if target_host_id.is_empty() {
        blockers.push("target_host_id is empty".to_string());
    }

    if artifact_ref.is_empty() {
        blockers.push("artifact_ref is empty".to_string());
    }

    if !mutation_policy.permits_mutation() {
        blockers.push(format!(
            "mutation_policy '{:?}' does not permit deploy",
            mutation_policy
        ));
    }

    // ExplicitApplyOnly requires the apply flag; AdmittedDispatchOrExplicitApplyOnly
    // would also accept an admitted dispatch, but admission here only checks the
    // apply gesture since dispatch admission is a separate concern.
    if mutation_policy == DeployMutationPolicy::ExplicitApplyOnly && !apply {
        blockers.push("explicit apply required (--apply)".to_string());
    }

    DeployAdmission {
        admitted: blockers.is_empty(),
        blockers,
    }
}

// ── Port trait ────────────────────────────────────────────────────────────────

/// Port for deploying build artifacts to target hosts.
///
/// Abstracts deployment, rollback, and health-check operations. Concrete
/// implementations (SSH, HTTP builder API, local install) live in adapter
/// crates. No I/O in this trait — all effects return through typed results.
pub trait DeployPort: Send + Sync {
    /// Deploy a build artifact to a target host.
    fn deploy_artifact(&self, request: &DeployRequest) -> DeployResult;

    /// Rollback to a previous version on the target host.
    fn rollback(&self, request: &DeployRequest) -> DeployResult;

    /// Verify a deployment is healthy on the target host.
    fn health_check(&self, request: &DeployRequest) -> DeployResult;
}

// ── Pure proof builder ────────────────────────────────────────────────────────

/// Build a deploy proof from operation results. Pure, no I/O.
pub fn build_deploy_proof(
    request: &DeployRequest,
    result: &DeployResult,
    admitted: bool,
    blockers: Vec<String>,
) -> DeployProof {
    let health_check_passed = match request.kind {
        DeployKind::HealthCheck => result.success,
        _ => result.kind == DeployKind::HealthCheck && result.success,
    };

    DeployProof {
        schema: DEPLOY_PROOF_SCHEMA,
        target_host_id: request.target_host_id.clone(),
        kind: request.kind,
        artifact_ref: result.artifact_ref.clone(),
        success: result.success,
        rolled_back: request.kind == DeployKind::Rollback && result.success,
        health_check_passed,
        mutation_policy: request.mutation_policy,
        admitted,
        blockers,
        output: result.output.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Mutation policy ─────────────────────────────────────────────────────

    #[test]
    fn deploy_mutation_policy_parse_read_only() {
        let policy = DeployMutationPolicy::parse("read_only");
        assert_eq!(policy, DeployMutationPolicy::ReadOnly);
        assert!(!policy.permits_mutation());
    }

    #[test]
    fn deploy_mutation_policy_parse_explicit_apply() {
        let policy = DeployMutationPolicy::parse("explicit_apply_only");
        assert_eq!(policy, DeployMutationPolicy::ExplicitApplyOnly);
        assert!(policy.permits_mutation());
    }

    #[test]
    fn deploy_mutation_policy_parse_admitted_dispatch() {
        let policy = DeployMutationPolicy::parse("admitted_dispatch_or_explicit_apply_only");
        assert_eq!(
            policy,
            DeployMutationPolicy::AdmittedDispatchOrExplicitApplyOnly
        );
        assert!(policy.permits_mutation());
    }

    #[test]
    fn deploy_mutation_policy_unknown_fails_closed() {
        let policy = DeployMutationPolicy::parse("something_unknown");
        assert_eq!(policy, DeployMutationPolicy::Unrecognised);
        assert!(!policy.permits_mutation());
    }

    // ── Admission ────────────────────────────────────────────────────────────

    #[test]
    fn admit_deploy_with_apply_and_explicit_policy_passes() {
        let r = admit_external_deploy(
            "staging-node-1",
            "sha256:abc123",
            DeployMutationPolicy::ExplicitApplyOnly,
            true,
        );
        assert!(r.admitted);
        assert!(r.blockers.is_empty());
    }

    #[test]
    fn admit_deploy_without_apply_blocked() {
        let r = admit_external_deploy(
            "staging-node-1",
            "sha256:abc123",
            DeployMutationPolicy::ExplicitApplyOnly,
            false,
        );
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("explicit apply")));
    }

    #[test]
    fn admit_deploy_read_only_blocked() {
        let r = admit_external_deploy(
            "staging-node-1",
            "sha256:abc123",
            DeployMutationPolicy::ReadOnly,
            true,
        );
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("mutation_policy")));
    }

    #[test]
    fn admit_deploy_empty_host_blocked() {
        let r = admit_external_deploy(
            "",
            "sha256:abc123",
            DeployMutationPolicy::ExplicitApplyOnly,
            true,
        );
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("target_host_id")));
    }

    #[test]
    fn admit_deploy_empty_artifact_blocked() {
        let r = admit_external_deploy(
            "staging-node-1",
            "",
            DeployMutationPolicy::ExplicitApplyOnly,
            true,
        );
        assert!(!r.admitted);
        assert!(r.blockers.iter().any(|b| b.contains("artifact_ref")));
    }

    #[test]
    fn admit_deploy_unrecognised_policy_blocked() {
        let r = admit_external_deploy(
            "staging-node-1",
            "sha256:abc123",
            DeployMutationPolicy::Unrecognised,
            true,
        );
        assert!(!r.admitted);
    }

    // ── Proof builder ────────────────────────────────────────────────────────

    fn sample_request() -> DeployRequest {
        DeployRequest {
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:abc123".to_string(),
            kind: DeployKind::Deploy,
            mutation_policy: DeployMutationPolicy::ExplicitApplyOnly,
            apply: true,
            health_check_argv: None,
            previous_artifact_ref: Some("sha256:def456".to_string()),
        }
    }

    #[test]
    fn build_proof_successful_deploy() {
        let request = sample_request();
        let result = DeployResult {
            kind: DeployKind::Deploy,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:abc123".to_string(),
            success: true,
            output: Some(DeployOutput {
                stdout: "deployed".to_string(),
                stderr: "".to_string(),
                exit_code: Some(0),
            }),
            error: None,
        };
        let proof = build_deploy_proof(&request, &result, true, vec![]);
        assert!(proof.success);
        assert!(!proof.rolled_back);
        assert!(!proof.health_check_passed);
        assert!(proof.admitted);
        assert_eq!(proof.target_host_id, "staging-node-1");
    }

    #[test]
    fn build_proof_rollback_marks_rolled_back() {
        let request = DeployRequest {
            kind: DeployKind::Rollback,
            ..sample_request()
        };
        let result = DeployResult {
            kind: DeployKind::Rollback,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:def456".to_string(),
            success: true,
            output: None,
            error: None,
        };
        let proof = build_deploy_proof(&request, &result, true, vec![]);
        assert!(proof.success);
        assert!(proof.rolled_back);
    }

    #[test]
    fn build_proof_health_check_passed() {
        let request = DeployRequest {
            kind: DeployKind::HealthCheck,
            ..sample_request()
        };
        let result = DeployResult {
            kind: DeployKind::HealthCheck,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:abc123".to_string(),
            success: true,
            output: None,
            error: None,
        };
        let proof = build_deploy_proof(&request, &result, true, vec![]);
        assert!(proof.health_check_passed);
    }

    #[test]
    fn build_proof_health_check_failed() {
        let request = DeployRequest {
            kind: DeployKind::HealthCheck,
            ..sample_request()
        };
        let result = DeployResult {
            kind: DeployKind::HealthCheck,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:abc123".to_string(),
            success: false,
            output: None,
            error: Some("connection refused".to_string()),
        };
        let proof = build_deploy_proof(&request, &result, false, vec!["unhealthy".to_string()]);
        assert!(!proof.success);
        assert!(!proof.health_check_passed);
        assert!(!proof.admitted);
        assert_eq!(proof.blockers, vec!["unhealthy".to_string()]);
    }

    #[test]
    fn build_proof_failed_deploy() {
        let request = sample_request();
        let result = DeployResult {
            kind: DeployKind::Deploy,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:abc123".to_string(),
            success: false,
            output: Some(DeployOutput {
                stdout: "".to_string(),
                stderr: "permission denied".to_string(),
                exit_code: Some(1),
            }),
            error: Some("permission denied".to_string()),
        };
        let proof = build_deploy_proof(&request, &result, false, vec!["deploy_failed".to_string()]);
        assert!(!proof.success);
        assert!(proof.output.is_some());
        assert_eq!(proof.output.unwrap().stderr, "permission denied");
    }

    // ── Port trait object safety ─────────────────────────────────────────────

    #[test]
    fn port_trait_object_safe() {
        struct MockDeployPort;

        impl DeployPort for MockDeployPort {
            fn deploy_artifact(&self, request: &DeployRequest) -> DeployResult {
                DeployResult {
                    kind: DeployKind::Deploy,
                    target_host_id: request.target_host_id.clone(),
                    artifact_ref: request.artifact_ref.clone(),
                    success: true,
                    output: Some(DeployOutput {
                        stdout: "mock deploy ok".to_string(),
                        stderr: "".to_string(),
                        exit_code: Some(0),
                    }),
                    error: None,
                }
            }

            fn rollback(&self, request: &DeployRequest) -> DeployResult {
                DeployResult {
                    kind: DeployKind::Rollback,
                    target_host_id: request.target_host_id.clone(),
                    artifact_ref: request.previous_artifact_ref.clone().unwrap_or_default(),
                    success: true,
                    output: None,
                    error: None,
                }
            }

            fn health_check(&self, _request: &DeployRequest) -> DeployResult {
                DeployResult {
                    kind: DeployKind::HealthCheck,
                    target_host_id: "mock".to_string(),
                    artifact_ref: "mock".to_string(),
                    success: true,
                    output: Some(DeployOutput {
                        stdout: "healthy".to_string(),
                        stderr: "".to_string(),
                        exit_code: Some(0),
                    }),
                    error: None,
                }
            }
        }

        let port: Box<dyn DeployPort> = Box::new(MockDeployPort);
        let request = sample_request();

        let deploy_result = port.deploy_artifact(&request);
        assert!(deploy_result.success);
        assert_eq!(deploy_result.output.unwrap().stdout, "mock deploy ok");

        let rollback_result = port.rollback(&request);
        assert!(rollback_result.success);
        assert_eq!(rollback_result.artifact_ref, "sha256:def456");

        let health_result = port.health_check(&request);
        assert!(health_result.success);
    }
}
