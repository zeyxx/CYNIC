//! Canonical construction of an admitted build-runner delivery.
//!
//! MCP, CLI, and a local Unix-socket client are projections over this use
//! case. None of those transports owns build admission or workload shape.

use cynic_protocol::{
    AgentBudgets, AgentCitizenshipRecord, AgentScope, AgentWorkloadDelivery,
    AgentWorkloadDeliveryStatus, BuildAgentRequest, BuildAgentRequestAdmission, MutationAdmission,
    admit_build_agent_request, admit_build_mutation, compose_build_agent_workload,
    plan_build_agent_request,
};
use serde::Serialize;

use crate::ports::build_workload_submission_port::{
    BuildWorkloadSubmissionPort, BuildWorkloadSubmissionReadBack,
};

pub struct BuildWorkloadSubmissionPlan {
    pub admission: BuildAgentRequestAdmission,
    pub delivery: Option<AgentWorkloadDelivery>,
}

pub struct BuildWorkloadSubmissionResult {
    pub read_back: BuildWorkloadSubmissionReadBack,
    pub read_back_violations: Vec<String>,
}

/// Transport-neutral proof produced after an admitted delivery attempt.
///
/// CLI and MCP may add their own request context, but must project this exact
/// decision rather than independently inferring delivery success from HTTP.
#[derive(Debug, Serialize)]
pub struct BuildWorkloadSubmissionReadBackReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutation_attempted: bool,
    pub delivery: AgentWorkloadDelivery,
    pub kernel_http_status: u16,
    pub delivery_read_back: serde_json::Value,
    pub delivery_read_back_verified: bool,
    pub delivery_read_back_violations: Vec<String>,
}

/// Explicit degraded outcome when the kernel transport cannot produce a
/// read-back. It is deliberately distinct from a failed proof after HTTP.
#[derive(Debug, Serialize)]
pub struct BuildWorkloadSubmissionBlockedReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutation_attempted: bool,
    pub delivery: AgentWorkloadDelivery,
    pub blockers: Vec<String>,
}

pub fn blocked_build_workload_submission(
    delivery: AgentWorkloadDelivery,
    blocker: String,
) -> BuildWorkloadSubmissionBlockedReport {
    BuildWorkloadSubmissionBlockedReport {
        schema: "cynic.build_workload_submission_read_back.v1",
        status: "blocked_disconnected",
        mutation_attempted: false,
        delivery,
        blockers: vec![blocker],
    }
}

impl BuildWorkloadSubmissionResult {
    pub fn into_read_back_report(
        self,
        delivery: AgentWorkloadDelivery,
    ) -> BuildWorkloadSubmissionReadBackReport {
        let delivery_read_back_verified = self.read_back_violations.is_empty();
        BuildWorkloadSubmissionReadBackReport {
            schema: "cynic.build_workload_submission_read_back.v1",
            status: if delivery_read_back_verified {
                "delivered"
            } else {
                "blocked_delivery_read_back"
            },
            mutation_attempted: (200..300).contains(&self.read_back.http_status),
            delivery,
            kernel_http_status: self.read_back.http_status,
            delivery_read_back: serde_json::Value::String(self.read_back.body),
            delivery_read_back_verified,
            delivery_read_back_violations: self.read_back_violations,
        }
    }
}

pub async fn submit_build_workload_submission(
    port: &dyn BuildWorkloadSubmissionPort,
    delivery: &AgentWorkloadDelivery,
) -> Result<BuildWorkloadSubmissionResult, String> {
    let read_back = port.submit(delivery).await?;
    let read_back_violations = if (200..300).contains(&read_back.http_status) {
        cynic_protocol::validate_agent_workload_submission_read_back(delivery, &read_back.body)
    } else {
        vec!["kernel_http_status_not_success".to_string()]
    };
    Ok(BuildWorkloadSubmissionResult {
        read_back,
        read_back_violations,
    })
}

pub fn plan_build_workload_submission(
    request: &BuildAgentRequest,
    registry_path: &str,
    workspace_root: &str,
    submitted_at_ms: u64,
) -> Result<BuildWorkloadSubmissionPlan, String> {
    let admission = if request.mutation_authorized {
        admit_build_agent_request(request)
    } else {
        plan_build_agent_request(request)
    };
    if admission.status == "blocked" || !request.mutation_authorized {
        return Ok(BuildWorkloadSubmissionPlan {
            admission,
            delivery: None,
        });
    }

    // Canonical admission gate — verify the requesting citizen has authority
    let citizen = AgentCitizenshipRecord {
        schema: "cynic.agent_citizenship.v1",
        agent_id: request.requested_by.clone(),
        scope: AgentScope::Crate,
        budgets: AgentBudgets {
            actions: 100,
            wall_seconds: 3600,
        },
        residual_risk: None,
        evidence: None,
    };
    let gate = admit_build_mutation(
        &citizen,
        Some(&AgentScope::Crate),
        &AgentBudgets {
            actions: 1,
            wall_seconds: 60,
        },
        &AgentScope::Crate,
        true,
        &request.desired_role,
        &request.desired_role,
    );
    if let MutationAdmission::Refused { codes } = &gate {
        let mut admission = admission;
        admission.status = "blocked".into();
        admission
            .blockers
            .push(format!("admission_gate_refused:{:?}", codes));
        return Ok(BuildWorkloadSubmissionPlan {
            admission,
            delivery: None,
        });
    }

    let transport = cynic_host::registry::resolve_build_runner_transport(Some(registry_path))?;
    let envelope = compose_build_agent_workload(request, &transport.node_id, workspace_root)
        .map_err(|blockers| blockers.join(","))?;
    Ok(BuildWorkloadSubmissionPlan {
        admission,
        delivery: Some(AgentWorkloadDelivery {
            schema: cynic_protocol::AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
            target_node_id: transport.node_id,
            submitted_by: request.requested_by.clone(),
            submitted_at_ms,
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
            envelope,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_only_never_composes_a_delivery() {
        let request = BuildAgentRequest {
            schema: cynic_protocol::BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "check-aaaaaaaaaaaa-agent".to_string(),
            requested_by: "agent".to_string(),
            source_commit: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            source: cynic_protocol::BuildSourceReference {
                schema: cynic_protocol::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: "ssh://forgejo/cynic.git".to_string(),
                revision: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                authority_ref: "infra/repo-remotes.json:forgejo".to_string(),
                snapshot: None,
            },
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: false,
            proof_requirements: vec![cynic_protocol::BUILD_GRANT_READ_BACK_PROOF.to_string()],
            release_publication: None,
            evidence_refs: vec!["source-proof".to_string()],
        };
        let plan =
            plan_build_workload_submission(&request, "/missing/registry.json", "/workspace", 1)
                .expect("plan-only admission");
        assert_eq!(plan.admission.status, "planned");
        assert!(plan.delivery.is_none());
    }
}
