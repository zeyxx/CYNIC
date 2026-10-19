//! Pure orchestration usecase for external deploy operations.
//!
//! WHY: composes the pure admission validator, the `DeployPort` trait, and the
//! pure proof builder into callable usecase functions. Callers inject any port
//! implementation (HTTP, local, mock) and receive typed proofs.
//! HOW: thin orchestration layer. `deploy_and_verify` calls `deploy_artifact`
//! then `health_check` on success. `rollback_on_failure` calls `rollback`.
//! NEGATIVE BOUNDARY: no std::process::Command, no SSH, no HTTP, no filesystem
//! I/O.

use crate::deploy::{
    admit_external_deploy, build_deploy_proof, DeployKind, DeployPort, DeployProof, DeployRequest,
};

impl AsRef<DeployRequest> for DeployRequest {
    fn as_ref(&self) -> &DeployRequest {
        self
    }
}

// ── Usecase: deploy and verify ────────────────────────────────────────────────

/// Deploy an artifact and verify it via health check.
///
/// Pure orchestration: delegates all I/O to the port. Calls
/// `port.deploy_artifact` first; on success calls `port.health_check`.
/// Returns a `DeployProof` describing the overall outcome.
pub fn deploy_and_verify(
    port: &dyn DeployPort,
    req: impl AsRef<DeployRequest>,
) -> DeployProof {
    let req = req.as_ref();
    let admission = admit_external_deploy(
        &req.target_host_id,
        &req.artifact_ref,
        req.mutation_policy,
        req.apply,
    );

    let deploy_result = port.deploy_artifact(req);

    let final_result = if deploy_result.success {
        port.health_check(req)
    } else {
        deploy_result.clone()
    };

    let mut blockers = admission.blockers.clone();
    if admission.admitted {
        if !final_result.success {
            if let Some(error) = &final_result.error {
                if !blockers.iter().any(|b| b == error) {
                    blockers.push(error.clone());
                }
            } else {
                let label = if deploy_result.success {
                    "health_check_failed"
                } else {
                    "deploy_failed"
                };
                if !blockers.iter().any(|b| b == label) {
                    blockers.push(label.to_string());
                }
            }
        }
    }

    build_deploy_proof(req, &final_result, admission.admitted, blockers)
}

// ── Usecase: rollback on failure ──────────────────────────────────────────────

/// Roll back to the previous artifact on failure.
///
/// Pure orchestration: delegates all I/O to the port via `port.rollback`.
/// Returns a `DeployProof` describing the rollback outcome.
pub fn rollback_on_failure(
    port: &dyn DeployPort,
    req: impl AsRef<DeployRequest>,
) -> DeployProof {
    let req = req.as_ref();
    let admission = admit_external_deploy(
        &req.target_host_id,
        &req.artifact_ref,
        req.mutation_policy,
        req.apply,
    );

    let result = port.rollback(req);

    // Preserve the request kind as Rollback so the proof marks rolled_back.
    let mut proof_request = req.clone();
    proof_request.kind = DeployKind::Rollback;
    if result.artifact_ref != req.artifact_ref && !result.artifact_ref.is_empty() {
        proof_request.artifact_ref = result.artifact_ref.clone();
    }

    let mut blockers = admission.blockers.clone();
    if admission.admitted && !result.success {
        if let Some(error) = &result.error {
            if !blockers.iter().any(|b| b == error) {
                blockers.push(error.clone());
            }
        } else if !blockers.iter().any(|b| b == "rollback_failed") {
            blockers.push("rollback_failed".to_string());
        }
    }

    build_deploy_proof(&proof_request, &result, admission.admitted, blockers)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy::{DeployMutationPolicy, DeployOutput};

    struct MockDeployPort {
        deploy_result: crate::deploy::DeployResult,
        health_result: crate::deploy::DeployResult,
        rollback_result: crate::deploy::DeployResult,
        calls: std::sync::Mutex<Vec<String>>,
    }

    impl DeployPort for MockDeployPort {
        fn deploy_artifact(&self, request: &DeployRequest) -> crate::deploy::DeployResult {
            self.calls.lock().unwrap().push("deploy".to_string());
            let mut r = self.deploy_result.clone();
            r.target_host_id = request.target_host_id.clone();
            r.artifact_ref = request.artifact_ref.clone();
            r
        }

        fn rollback(&self, request: &DeployRequest) -> crate::deploy::DeployResult {
            self.calls.lock().unwrap().push("rollback".to_string());
            let mut r = self.rollback_result.clone();
            r.target_host_id = request.target_host_id.clone();
            if r.artifact_ref.is_empty() {
                r.artifact_ref = request
                    .previous_artifact_ref
                    .clone()
                    .unwrap_or_else(|| request.artifact_ref.clone());
            }
            r
        }

        fn health_check(&self, request: &DeployRequest) -> crate::deploy::DeployResult {
            self.calls.lock().unwrap().push("health".to_string());
            let mut r = self.health_result.clone();
            r.target_host_id = request.target_host_id.clone();
            r.artifact_ref = request.artifact_ref.clone();
            r
        }
    }

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

    fn ok_deploy() -> crate::deploy::DeployResult {
        crate::deploy::DeployResult {
            kind: DeployKind::Deploy,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:abc123".to_string(),
            success: true,
            output: Some(DeployOutput {
                stdout: "deployed".to_string(),
                stderr: String::new(),
                exit_code: Some(0),
            }),
            error: None,
        }
    }

    fn ok_health() -> crate::deploy::DeployResult {
        crate::deploy::DeployResult {
            kind: DeployKind::HealthCheck,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:abc123".to_string(),
            success: true,
            output: Some(DeployOutput {
                stdout: "healthy".to_string(),
                stderr: String::new(),
                exit_code: Some(0),
            }),
            error: None,
        }
    }

    fn fail_deploy() -> crate::deploy::DeployResult {
        crate::deploy::DeployResult {
            kind: DeployKind::Deploy,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:abc123".to_string(),
            success: false,
            output: Some(DeployOutput {
                stdout: String::new(),
                stderr: "boom".to_string(),
                exit_code: Some(1),
            }),
            error: Some("boom".to_string()),
        }
    }

    fn fail_health() -> crate::deploy::DeployResult {
        crate::deploy::DeployResult {
            kind: DeployKind::HealthCheck,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:abc123".to_string(),
            success: false,
            output: None,
            error: Some("unhealthy".to_string()),
        }
    }

    fn ok_rollback() -> crate::deploy::DeployResult {
        crate::deploy::DeployResult {
            kind: DeployKind::Rollback,
            target_host_id: "staging-node-1".to_string(),
            artifact_ref: "sha256:def456".to_string(),
            success: true,
            output: None,
            error: None,
        }
    }

    #[test]
    fn deploy_and_verify_success_calls_deploy_then_health() {
        let port = MockDeployPort {
            deploy_result: ok_deploy(),
            health_result: ok_health(),
            rollback_result: ok_rollback(),
            calls: std::sync::Mutex::new(Vec::new()),
        };
        let req = sample_request();
        let proof = deploy_and_verify(&port, req);
        assert!(proof.success);
        assert!(proof.health_check_passed);
        assert!(!proof.rolled_back);
        assert_eq!(proof.kind, DeployKind::Deploy);
        let calls = port.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["deploy".to_string(), "health".to_string()]);
    }

    #[test]
    fn deploy_and_verify_accepts_borrowed_request() {
        let port = MockDeployPort {
            deploy_result: ok_deploy(),
            health_result: ok_health(),
            rollback_result: ok_rollback(),
            calls: std::sync::Mutex::new(Vec::new()),
        };
        let req = sample_request();
        let proof = deploy_and_verify(&port, &req);
        assert!(proof.success);
        assert!(proof.health_check_passed);
    }

    #[test]
    fn deploy_and_verify_deploy_failure_skips_health() {
        let port = MockDeployPort {
            deploy_result: fail_deploy(),
            health_result: ok_health(),
            rollback_result: ok_rollback(),
            calls: std::sync::Mutex::new(Vec::new()),
        };
        let req = sample_request();
        let proof = deploy_and_verify(&port, req);
        assert!(!proof.success);
        assert!(!proof.health_check_passed);
        let calls = port.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["deploy".to_string()]);
    }

    #[test]
    fn deploy_and_verify_health_failure_marks_unhealthy() {
        let port = MockDeployPort {
            deploy_result: ok_deploy(),
            health_result: fail_health(),
            rollback_result: ok_rollback(),
            calls: std::sync::Mutex::new(Vec::new()),
        };
        let req = sample_request();
        let proof = deploy_and_verify(&port, req);
        assert!(!proof.success);
        assert!(!proof.health_check_passed);
        let calls = port.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["deploy".to_string(), "health".to_string()]);
    }

    #[test]
    fn rollback_on_failure_calls_rollback_and_marks_rolled_back() {
        let port = MockDeployPort {
            deploy_result: ok_deploy(),
            health_result: ok_health(),
            rollback_result: ok_rollback(),
            calls: std::sync::Mutex::new(Vec::new()),
        };
        let req = sample_request();
        let proof = rollback_on_failure(&port, req);
        assert!(proof.success);
        assert!(proof.rolled_back);
        assert_eq!(proof.kind, DeployKind::Rollback);
        let calls = port.calls.lock().unwrap().clone();
        assert_eq!(calls, vec!["rollback".to_string()]);
    }

    #[test]
    fn rollback_on_failure_accepts_borrowed_request() {
        let port = MockDeployPort {
            deploy_result: ok_deploy(),
            health_result: ok_health(),
            rollback_result: ok_rollback(),
            calls: std::sync::Mutex::new(Vec::new()),
        };
        let req = sample_request();
        let proof = rollback_on_failure(&port, &req);
        assert!(proof.rolled_back);
    }
}
