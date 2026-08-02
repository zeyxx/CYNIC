use serde::{Deserialize, Serialize};

use crate::{
    AGENT_DELEGATION_ENVELOPE_SCHEMA, AGENT_WORKLOAD_ENVELOPE_SCHEMA, AgentBoundedAction,
    AgentBudget, AgentCapabilityGrant, AgentDelegationEnvelope, AgentEvidenceRef,
    AgentWorkloadEnvelope, EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization,
    MutationContract, POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority,
    PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
};

pub const BUILD_AGENT_REQUEST_SCHEMA: &str = "cynic.build.agent_request.v1";
pub const BUILD_AGENT_REQUEST_ADMISSION_SCHEMA: &str = "cynic.build.agent_request_admission.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildAgentRequest {
    pub schema: String,
    pub request_id: String,
    pub requested_by: String,
    pub source_commit: String,
    pub action: String,
    pub desired_role: String,
    pub mutation_authorized: bool,
    pub proof_requirements: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildAgentRequestAdmission {
    pub schema: String,
    pub status: String,
    pub request_id: String,
    pub source_commit: String,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub fn plan_build_agent_request(request: &BuildAgentRequest) -> BuildAgentRequestAdmission {
    assess(request, false)
}

pub fn admit_build_agent_request(request: &BuildAgentRequest) -> BuildAgentRequestAdmission {
    assess(request, true)
}

pub fn compose_build_agent_workload(
    request: &BuildAgentRequest,
    target_node_id: &str,
    workspace_root: &str,
) -> Result<AgentWorkloadEnvelope, Vec<String>> {
    let admission = admit_build_agent_request(request);
    let mut blockers = admission.blockers;
    if target_node_id.trim().is_empty() {
        blockers.push("build_agent_workload_target_node_missing".to_string());
    }
    if workspace_root.trim().is_empty() {
        blockers.push("build_agent_workload_directory_missing".to_string());
    }
    if !blockers.is_empty() {
        return Err(blockers);
    }
    let argv = match request.action.as_str() {
        "make:check" => vec!["make".to_string(), "check".to_string()],
        "make:build" => vec!["make".to_string(), "build".to_string()],
        _ => return Err(vec!["build_agent_request_action_not_allowed".to_string()]),
    };
    let input_digest = digest_json(&serde_json::json!({
        "repository_id": "cynic",
        "source_commit": request.source_commit,
    }))
    .map_err(|_| vec!["build_agent_workload_input_digest_failed".to_string()])?;
    let request_digest = digest_json(request)
        .map_err(|_| vec!["build_agent_workload_request_digest_failed".to_string()])?;
    let suffix = request_digest.trim_start_matches("sha256:");
    let short = suffix.chars().take(24).collect::<String>();
    let workload_id = format!("build-{short}");
    let action = AgentBoundedAction {
        argv,
        working_directory: Some(format!(
            "{}/{}",
            workspace_root.trim_end_matches('/'),
            workload_id
        )),
        input_digest: Some(input_digest.clone()),
        mutates: true,
        requires_privilege: false,
        stop_condition: "stop on first non-zero gate exit and retain read-back evidence"
            .to_string(),
        rollback_intent: "discard the isolated source workspace and build artifacts".to_string(),
    };
    let plan_digest = digest_json(&action)
        .map_err(|_| vec!["build_agent_workload_plan_digest_failed".to_string()])?;
    let policy_digest = digest_json(&serde_json::json!({
        "id": "build.repository_validation",
        "version": "v1",
        "actions": ["make:check", "make:build"],
        "authority": "runtime_principal",
    }))
    .map_err(|_| vec!["build_agent_workload_policy_digest_failed".to_string()])?;
    let proofs = request
        .proof_requirements
        .iter()
        .map(|gate| RequiredProof {
            gate: gate.clone(),
            subject: request.source_commit.clone(),
        })
        .collect::<Vec<_>>();
    let mutation = MutationContract {
        kind: "repository_validation".to_string(),
        scope: PolicyScope {
            domain: "build".to_string(),
            target: request.source_commit.clone(),
        },
        mutates: true,
        requires_privilege: false,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::RuntimePrincipal,
        preconditions: vec!["source_commit_observed".to_string()],
        rollback_intent: action.rollback_intent.clone(),
        expected_proofs: proofs.clone(),
    };
    Ok(AgentWorkloadEnvelope {
        schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.to_string(),
        workload_id: workload_id.clone(),
        idempotency_key: format!("build-request-{short}"),
        plan_digest: plan_digest.clone(),
        action,
        delegation: AgentDelegationEnvelope {
            schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
            delegation_id: format!("delegation-{short}"),
            node_id: target_node_id.to_string(),
            agent_id: request.requested_by.clone(),
            parent_agent_id: None,
            depth: 0,
            objective: format!(
                "execute {} for source commit {}",
                request.action, request.source_commit
            ),
            authority_snapshot: AgentEvidenceRef {
                kind: "build_agent_request".to_string(),
                subject: request.request_id.clone(),
                digest: request_digest.clone(),
                lineage: None,
            },
            context_refs: vec![AgentEvidenceRef {
                kind: "source_commit".to_string(),
                subject: request.source_commit.clone(),
                digest: input_digest,
                lineage: None,
            }],
            capabilities: vec![AgentCapabilityGrant {
                id: "repository_validation".to_string(),
                mutates: true,
            }],
            budget: AgentBudget {
                context_tokens: 0,
                actions: 1,
                children: 0,
                wall_time_ms: 900_000,
            },
            expected_proofs: request.proof_requirements.clone(),
        },
        parent_delegation: None,
        policy_admission: PolicyAdmissionReport {
            schema: POLICY_ADMISSION_REPORT_SCHEMA.to_string(),
            policy_set: PolicySetRef {
                id: "build.repository_validation".to_string(),
                version: "v1".to_string(),
                digest: policy_digest,
            },
            intent: PolicyIntentRef {
                id: request.request_id.clone(),
                kind: "repository_validation".to_string(),
                actor: request.requested_by.clone(),
                subject: request.source_commit.clone(),
                digest: request_digest,
                plan_digest,
            },
            mutation: mutation.clone(),
            decision: PolicyDecision::Allow {
                required_authority: PolicyAuthority::RuntimePrincipal,
                required_proofs: proofs,
            },
        },
        execution_authorization: ExecutionAuthorization {
            schema: EXECUTION_AUTHORIZATION_SCHEMA.to_string(),
            actor: request.requested_by.clone(),
            authority: PolicyAuthority::RuntimePrincipal,
            explicit_apply: request.mutation_authorized,
            satisfied_preconditions: vec!["source_commit_observed".to_string()],
        },
    })
}

fn assess(
    request: &BuildAgentRequest,
    require_mutation_authority: bool,
) -> BuildAgentRequestAdmission {
    let mut blockers = Vec::new();
    if request.schema != BUILD_AGENT_REQUEST_SCHEMA {
        blockers.push("build_agent_request_schema_mismatch".to_string());
    }
    if request.request_id.trim().is_empty() {
        blockers.push("build_agent_request_id_missing".to_string());
    }
    if request.requested_by.trim().is_empty() {
        blockers.push("build_agent_request_principal_missing".to_string());
    }
    if !is_commit_id(&request.source_commit) {
        blockers.push("build_agent_request_source_commit_invalid".to_string());
    }
    if !matches!(request.action.as_str(), "make:check" | "make:build") {
        blockers.push("build_agent_request_action_not_allowed".to_string());
    }
    if request.desired_role != "build_runner" {
        blockers.push("build_agent_request_role_mismatch".to_string());
    }
    if require_mutation_authority && !request.mutation_authorized {
        blockers.push("build_agent_request_dispatch_not_authorized".to_string());
    }
    if request.proof_requirements.is_empty() {
        blockers.push("build_agent_request_proof_requirements_missing".to_string());
    }
    if request.evidence_refs.is_empty() {
        blockers.push("build_agent_request_source_evidence_missing".to_string());
    }
    BuildAgentRequestAdmission {
        schema: BUILD_AGENT_REQUEST_ADMISSION_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            if require_mutation_authority {
                "admitted"
            } else {
                "planned"
            }
        } else {
            "blocked"
        }
        .to_string(),
        request_id: request.request_id.clone(),
        source_commit: request.source_commit.clone(),
        blockers,
        evidence_refs: request.evidence_refs.clone(),
    }
}

fn is_commit_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(mutation_authorized: bool) -> BuildAgentRequest {
        BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "validate-abc".to_string(),
            requested_by: "codex-session-1".to_string(),
            source_commit: "a".repeat(40),
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized,
            proof_requirements: vec![
                "canonical_gate_report".to_string(),
                "source_commit_read_back".to_string(),
            ],
            evidence_refs: vec!["forgejo:refs/heads/change@aaaaaaaa".to_string()],
        }
    }

    #[test]
    fn planning_does_not_require_dispatch_authority() {
        assert_eq!(plan_build_agent_request(&request(false)).status, "planned");
    }

    #[test]
    fn dispatch_requires_explicit_authority() {
        let admission = admit_build_agent_request(&request(false));
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"build_agent_request_dispatch_not_authorized".to_string())
        );
    }

    #[test]
    fn shell_or_arbitrary_build_actions_are_not_admitted() {
        let mut request = request(true);
        request.action = "bash -c make check".to_string();
        assert!(
            admit_build_agent_request(&request)
                .blockers
                .contains(&"build_agent_request_action_not_allowed".to_string())
        );
    }

    #[test]
    fn canonical_artifact_build_is_admissible() {
        let mut request = request(true);
        request.action = "make:build".to_string();
        assert_eq!(admit_build_agent_request(&request).status, "admitted");
    }

    #[test]
    fn dispatch_composes_one_admitted_content_addressed_workload() {
        let envelope = compose_build_agent_workload(
            &request(true),
            "cynic-builder",
            "/var/lib/cynic/source/workspaces/build-test",
        )
        .unwrap();
        assert_eq!(envelope.action.argv, vec!["make", "check"]);
        assert_eq!(envelope.delegation.node_id, "cynic-builder");
        assert!(crate::admit_agent_workload(&envelope).admitted());
    }
}
