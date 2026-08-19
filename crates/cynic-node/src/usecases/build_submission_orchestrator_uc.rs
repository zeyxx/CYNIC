//! Build submission orchestrator — admission + grant + submission.
//!
//! This is the canonical path for triggering a build:
//!   1. admit the BuildAgentRequest (cynic_protocol::admit_build_agent_request)
//!   2. compose the AgentWorkloadEnvelope (cynic_protocol::compose_build_agent_workload)
//!   3. issue a signed ed25519 BuildWorkloadGrant (issue_signed_build_workload_grant)
//!   4. submit the delivery to the kernel (BuildWorkloadSubmissionPort::submit)
//!
//! CLI and MCP are projections over this orchestrator. Neither owns admission,
//! grant signing, or submission policy.

use cynic_protocol::{
    admit_build_agent_request, compose_build_agent_workload, AgentWorkloadDelivery,
    AgentWorkloadDeliveryStatus, BuildAgentRequest, BuildAgentRequestAdmission,
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
    registry_path: &str,
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
    let transport = match resolve_build_runner_node_id(registry_path) {
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

    let delivery = AgentWorkloadDelivery {
        schema: cynic_protocol::AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
        target_node_id: transport,
        submitted_by: request.requested_by.clone(),
        submitted_at_ms: now_unix_s * 1000,
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
    let durable_record = cynic_protocol::DurableAgentWorkloadRecord {
        schema: cynic_protocol::DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
        revision: 1,
        previous_record_digest: None,
        record_digest: format!("sha256:{}", &request.request_id),
        record: cynic_protocol::AgentWorkloadRecord {
            schema: cynic_protocol::AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
            envelope: envelope.clone(),
            state: cynic_protocol::AgentWorkloadState::Dispatched,
            claim: None,
            last_fencing_token: 0,
            cancellation_generation: 0,
            consumed_actions: 0,
            completion: None,
        },
        committed_at_ms: now_unix_s * 1000,
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

/// Resolve the build runner node id from the registry.
fn resolve_build_runner_node_id(registry_path: &str) -> Result<String, String> {
    let registry = cynic_host::build_runner_harness::registry::read_registry(registry_path)?;
    let nodes = registry
        .get("nodes")
        .and_then(|n| n.as_array())
        .ok_or_else(|| "registry: no nodes array".to_string())?;

    let builder = nodes
        .iter()
        .find(|node| {
            node.get("role")
                .and_then(|r| r.as_str())
                .map(|r| r == "build_runner")
                .unwrap_or(false)
        })
        .ok_or_else(|| "build_runner node not found".to_string())?;

    builder
        .get("id")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "build_runner node id missing".to_string())
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
        ) -> Result<crate::ports::build_workload_submission_port::BuildWorkloadSubmissionReadBack, String>
        {
            if self.should_fail {
                Err("submit failed".into())
            } else {
                Ok(crate::ports::build_workload_submission_port::BuildWorkloadSubmissionReadBack {
                    http_status: 202,
                    body: r#"{"status":"accepted"}"#.into(),
                })
            }
        }
    }

    fn valid_request() -> BuildAgentRequest {
        BuildAgentRequest {
            schema: cynic_protocol::BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "test-request-001".to_string(),
            requested_by: "test-agent".to_string(),
            source_commit: "a".repeat(40),
            source: cynic_protocol::BuildSourceReference {
                schema: cynic_protocol::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
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
}
