use crate::{
    AGENT_DELEGATION_ENVELOPE_SCHEMA, AGENT_WORKLOAD_ENVELOPE_SCHEMA, AgentBoundedAction,
    AgentBudget, AgentCapabilityGrant, AgentDelegationEnvelope, AgentEvidenceRef,
    AgentWorkloadEnvelope, EXECUTION_AUTHORIZATION_SCHEMA, ExecutionAuthorization,
    MutationContract, POLICY_ADMISSION_REPORT_SCHEMA, PolicyAdmissionReport, PolicyAuthority,
    PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
};

use super::{BuildAgentRequest, admit_build_agent_request};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BUILD_AGENT_REQUEST_SCHEMA;

    #[test]
    fn dispatch_composes_one_admitted_content_addressed_workload() {
        let request = BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "validate-abc".to_string(),
            requested_by: "codex-session-1".to_string(),
            source_commit: "a".repeat(40),
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: true,
            proof_requirements: vec!["canonical_gate_report".to_string()],
            evidence_refs: vec!["forgejo:refs/heads/change@aaaaaaaa".to_string()],
        };

        let envelope = compose_build_agent_workload(
            &request,
            "cynic-builder",
            "/var/lib/cynic/source/workspaces/build-test",
        )
        .unwrap();

        assert_eq!(envelope.action.argv, vec!["make", "check"]);
        assert_eq!(envelope.delegation.node_id, "cynic-builder");
        assert!(crate::admit_agent_workload(&envelope).admitted());
    }
}
