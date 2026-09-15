//! Build submission orchestrator — admission + grant + submission.
//!
//! This is the canonical path for triggering a build:
//!   1. admit the BuildAgentRequest (cynic_build::admit_build_agent_request)
//!   2. compose the AgentWorkloadEnvelope (cynic_build::compose_build_agent_workload)
//!   3. issue a signed ed25519 BuildWorkloadGrant (issue_signed_build_workload_grant)
//!   4. submit the delivery to the kernel (BuildWorkloadSubmissionPort::submit)
//!
//! CLI and MCP are projections over this orchestrator. Neither owns admission,
//! grant signing, or submission policy.

use cynic_build::{
    BuildAgentRequest,
    BuildAgentRequestAdmission,
    BuildRunnerRouteDecision,
    BuildRunnerRoutePath,
    BuildRunnerRouteState,
    admit_build_agent_request,
    compose_build_agent_workload,
};
use cynic_workload::{
    AgentWorkloadDelivery,
    AgentWorkloadDeliveryStatus,
    AgentWorkloadState,
    claim_agent_workload,
    consume_agent_workload_action,
};
use cynic_protocol::{
    LeaseGuard,
};
use serde::Serialize;

use crate::ports::build_workload_grant_port::BuildWorkloadGrantSignerPort;
use crate::ports::build_workload_submission_port::BuildWorkloadSubmissionPort;
use crate::usecases::build_workload_grant_uc::issue_signed_build_workload_grant;

/// Full orchestration plan: admission + grant + submission.
#[derive(Debug, Serialize)]
pub struct BuildSubmissionPlan {
    pub schema: String,
    pub status: &'static str,
    pub admission: BuildAgentRequestAdmission,
    pub delivery: Option<AgentWorkloadDelivery>,
    pub grant: Option<GrantReport>,
    pub submission: Option<SubmissionReport>,
    pub blockers: Vec<String>,
}

/// Grant report (subset of SignedBuildWorkloadGrant for projection).
#[derive(Debug, Serialize)]
pub struct GrantReport {
    pub schema: String,
    pub grant_id: String,
    pub authority_key_id: String,
    pub workload_id: String,
    pub source_commit: String,
    pub expires_at_unix_s: u64,
}

/// Submission report (read-back from kernel).
#[derive(Debug, Serialize)]
pub struct SubmissionReport {
    pub http_status: u16,
    pub body: String,
}

/// Orchestrate: admit → grant → submit.
pub async fn orchestrate_build_submission(
    request: &BuildAgentRequest,
    route: &BuildRunnerRouteDecision,
    route_node_id: &str,
    workspace_root: &str,
    signer: &dyn BuildWorkloadGrantSignerPort,
    submitter: &dyn BuildWorkloadSubmissionPort,
    now_unix_s: u64,
) -> Result<BuildSubmissionPlan, String> {
    // Step 1: Admit
    let admission = admit_build_agent_request(request);
    if admission.status == "blocked" {
        return Ok(BuildSubmissionPlan {
            schema: "cynic.node.build_submission_plan.v1".to_string(),
            status: "blocked",
            admission,
            delivery: None,
            grant: None,
            submission: None,
            blockers: vec!["admission_blocked".to_string()],
        });
    }

    // Step 2: Compose workload envelope
    let transport = match resolve_runtime_fenced_build_runner(route, route_node_id) {
        Ok(id) => id,
        Err(e) => {
            return Ok(BuildSubmissionPlan {
                schema: "cynic.node.build_submission_plan.v1".to_string(),
                status: "blocked",
                admission,
                delivery: None,
                grant: None,
                submission: None,
                blockers: vec![format!("transport_resolution_failed:{e}")],
            });
        }
    };

    let envelope = match compose_build_agent_workload(request, &transport, workspace_root) {
        Ok(env) => env,
        Err(blockers) => {
            return Ok(BuildSubmissionPlan {
                schema: "cynic.node.build_submission_plan.v1".to_string(),
                status: "blocked",
                admission,
                delivery: None,
                grant: None,
                submission: None,
                blockers,
            });
        }
    };

    // Step 2b: Claim the workload so the grant can be issued
    let base_record = cynic_workload::AgentWorkloadRecord {
        schema: cynic_workload::AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        envelope: envelope.clone(),
        state: AgentWorkloadState::Dispatched,
        claim: None,
        last_fencing_token: 0,
        cancellation_generation: 0,
        consumed_actions: 0,
        completion: None,
    };
    let lease_guard = LeaseGuard {
        agent_id: request.requested_by.clone(),
        target: envelope.workload_id.clone(),
        lease_id: format!("claim-{}", envelope.workload_id),
        fencing_token: 1,
    };
    let now_ms = now_unix_s * 1000;
    let claimed = match claim_agent_workload(
        &base_record,
        lease_guard.clone(),
        now_ms + 300_000,
        now_ms,
        3_600_000,
    ) {
        Ok(r) => r,
        Err(violation) => {
            return Ok(BuildSubmissionPlan {
                schema: "cynic.node.build_submission_plan.v1".to_string(),
                status: "blocked",
                admission,
                delivery: None,
                grant: None,
                submission: None,
                blockers: vec![format!(
                    "claim_refused:{}:{}",
                    violation.code, violation.message
                )],
            });
        }
    };
    let consumed = match consume_agent_workload_action(&claimed, &lease_guard, now_ms + 1) {
        Ok(r) => r,
        Err(violation) => {
            return Ok(BuildSubmissionPlan {
                schema: "cynic.node.build_submission_plan.v1".to_string(),
                status: "blocked",
                admission,
                delivery: None,
                grant: None,
                submission: None,
                blockers: vec![format!(
                    "consume_refused:{}:{}",
                    violation.code, violation.message
                )],
            });
        }
    };

    let delivery = AgentWorkloadDelivery {
        schema: cynic_workload::AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
        target_node_id: transport,
        submitted_by: request.requested_by.clone(),
        submitted_at_ms: now_ms,
        status: AgentWorkloadDeliveryStatus::Pending,
        accepted_at_ms: None,
        accepted_record_digest: None,
        acceptance_lease: None,
        terminal_at_ms: None,
        terminal_record_digest: None,
        terminal_record: None,
        completion: None,
        completion_evidence: Vec::new(),
        evidence_refs: request.evidence_refs.clone(),
        envelope: envelope.clone(),
    };

    // Step 3: Issue signed grant
    let grant_record = cynic_workload::AgentWorkloadRecord {
        schema: cynic_workload::AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        envelope: envelope.clone(),
        state: consumed.state,
        claim: consumed.claim,
        last_fencing_token: consumed.last_fencing_token,
        cancellation_generation: consumed.cancellation_generation,
        consumed_actions: consumed.consumed_actions,
        completion: None,
    };
    let durable_record = cynic_workload::DurableAgentWorkloadRecord {
        schema: cynic_workload::DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        revision: 1,
        previous_record_digest: None,
        record_digest: cynic_protocol::digest_json(&grant_record)
            .map_err(|_| "grant_record_digest_failed".to_string())?,
        record: grant_record,
        committed_at_ms: now_ms,
    };

    let grant = match issue_signed_build_workload_grant(
        &durable_record,
        &request.requested_by.clone(),
        now_unix_s,
        signer,
    ) {
        Ok(signed) => Some(GrantReport {
            schema: signed.schema,
            grant_id: signed.grant.grant_id,
            authority_key_id: signed.authority_key_id,
            workload_id: signed.grant.workload_id,
            source_commit: signed.grant.source_commit,
            expires_at_unix_s: signed.grant.expires_at_unix_s,
        }),
        Err(blockers) => {
            return Ok(BuildSubmissionPlan {
                schema: "cynic.node.build_submission_plan.v1".to_string(),
                status: "blocked",
                admission,
                delivery: Some(delivery),
                grant: None,
                submission: None,
                blockers,
            });
        }
    };

    // Step 4: Submit to kernel
    let submission = match submitter.submit(&delivery).await {
        Ok(read_back) => Some(SubmissionReport {
            http_status: read_back.http_status,
            body: read_back.body,
        }),
        Err(e) => {
            return Ok(BuildSubmissionPlan {
                schema: "cynic.node.build_submission_plan.v1".to_string(),
                status: "blocked",
                admission,
                delivery: Some(delivery),
                grant,
                submission: None,
                blockers: vec![format!("submission_failed:{e}")],
            });
        }
    };

    Ok(BuildSubmissionPlan {
        schema: "cynic.node.build_submission_plan.v1".to_string(),
        status: "submitted",
        admission,
        delivery: Some(delivery),
        grant,
        submission,
        blockers: vec![],
    })
}

/// Normal workload delivery consumes the canonical route decision. It never
/// re-selects a Builder from a registry role declaration.
fn resolve_runtime_fenced_build_runner(
    route: &BuildRunnerRouteDecision,
    route_node_id: &str,
) -> Result<String, String> {
    if route_node_id.trim().is_empty() {
        return Err("build_runner_route_node_id_missing".to_string());
    }
    if route.node_id != route_node_id {
        return Err("build_runner_route_node_id_mismatch".to_string());
    }
    if route.state != BuildRunnerRouteState::RuntimeFenced
        || route.next_path != Some(BuildRunnerRoutePath::RuntimeDelivery)
        || !route.blockers.is_empty()
        || !route.residual_unknowns.is_empty()
    {
        return Err("build_runner_runtime_fence_required".to_string());
    }
    Ok(route_node_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeSigner {
        key_id: String,
    }

    impl BuildWorkloadGrantSignerPort for FakeSigner {
        fn authority_key_id(&self) -> &str {
            &self.key_id
        }
        fn sign(&self, _payload: &[u8]) -> Result<String, String> {
            Ok("a".repeat(128))
        }
    }

    struct FakeSubmitter {
        should_fail: bool,
    }

    #[async_trait::async_trait]
    impl BuildWorkloadSubmissionPort for FakeSubmitter {
        async fn submit(
            &self,
            _delivery: &AgentWorkloadDelivery,
        ) -> Result<
            crate::ports::build_workload_submission_port::BuildWorkloadSubmissionReadBack,
            String,
        > {
            if self.should_fail {
                Err("submit failed".into())
            } else {
                Ok(
                    crate::ports::build_workload_submission_port::BuildWorkloadSubmissionReadBack {
                        http_status: 202,
                        body: r#"{"status":"accepted"}"#.into(),
                    },
                )
            }
        }
    }

    fn valid_request() -> BuildAgentRequest {
        BuildAgentRequest {
            schema: cynic_build::BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "test-request-001".to_string(),
            requested_by: "test-agent".to_string(),
            source_commit: "a".repeat(40),
            source: cynic_build::BuildSourceReference {
                schema: cynic_build::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://forgejo/cynic.git".to_string(),
                revision: "a".repeat(40),
                authority_ref: "infra/repo-remotes.json:forgejo".to_string(),
                snapshot: None,
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec!["build_grant_read_back".to_string()],
            release_publication: None,
            evidence_refs: vec!["source-proof".to_string()],
        }
    }

    fn route(
        state: BuildRunnerRouteState,
        next_path: Option<BuildRunnerRoutePath>,
    ) -> BuildRunnerRouteDecision {
        BuildRunnerRouteDecision {
            schema: "cynic.build_runner.route_convergence.v1".to_string(),
            node_id: "cynic-builder".to_string(),
            state,
            next_path,
            mutates: false,
            blockers: Vec::new(),
            residual_unknowns: Vec::new(),
        }
    }

    #[test]
    fn orchestrate_admits_valid_request() {
        let request = valid_request();
        let admission = admit_build_agent_request(&request);
        assert_eq!(admission.status, "admitted");
    }

    #[test]
    fn orchestrate_blocks_invalid_commit() {
        let mut request = valid_request();
        request.source_commit = "short".to_string();
        let admission = admit_build_agent_request(&request);
        assert_eq!(admission.status, "blocked");
    }

    #[test]
    fn orchestrate_completes_admission_grant_submission_with_fakes() {
        let request = valid_request();
        let route = route(
            BuildRunnerRouteState::RuntimeFenced,
            Some(BuildRunnerRoutePath::RuntimeDelivery),
        );
        let signer = FakeSigner {
            key_id: format!("sha256:{}", "a".repeat(64)),
        };
        let submitter = FakeSubmitter { should_fail: false };
        let runtime = tokio::runtime::Runtime::new().expect("async runtime");
        let plan = runtime
            .block_on(orchestrate_build_submission(
                &request,
                &route,
                "cynic-builder",
                "/workspace",
                &signer,
                &submitter,
                1724000000,
            ))
            .expect("orchestration succeeds");
        assert_eq!(plan.status, "submitted");
        assert!(plan.delivery.is_some());
        assert!(plan.grant.is_some());
        assert!(plan.submission.is_some());
        assert!(plan.blockers.is_empty());
    }

    #[test]
    fn only_runtime_fenced_delivery_resolves_a_build_runner() {
        let fenced = route(
            BuildRunnerRouteState::RuntimeFenced,
            Some(BuildRunnerRoutePath::RuntimeDelivery),
        );
        assert_eq!(
            resolve_runtime_fenced_build_runner(&fenced, "cynic-builder"),
            Ok("cynic-builder".to_string())
        );
        assert_eq!(
            resolve_runtime_fenced_build_runner(&fenced, "other-builder"),
            Err("build_runner_route_node_id_mismatch".to_string())
        );
        let seed = route(
            BuildRunnerRouteState::SeedProved,
            Some(BuildRunnerRoutePath::SshSeed),
        );
        assert_eq!(
            resolve_runtime_fenced_build_runner(&seed, "cynic-builder"),
            Err("build_runner_runtime_fence_required".to_string())
        );
    }

    #[test]
    fn runtime_delivery_rejects_blockers_and_unknowns() {
        let mut blocked = route(
            BuildRunnerRouteState::RuntimeFenced,
            Some(BuildRunnerRoutePath::RuntimeDelivery),
        );
        blocked
            .residual_unknowns
            .push("coordination_stale".to_string());
        assert!(resolve_runtime_fenced_build_runner(&blocked, "cynic-builder").is_err());
        blocked.residual_unknowns.clear();
        blocked.blockers.push("route_refuted".to_string());
        assert!(resolve_runtime_fenced_build_runner(&blocked, "cynic-builder").is_err());
    }
}
