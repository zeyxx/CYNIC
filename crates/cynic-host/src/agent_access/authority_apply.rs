use cynic_protocol::{
    ExecutionAuthorization, MutationContract, PolicyAdmissionReport, PolicyAuthority,
    PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
};
use serde::Serialize;

use crate::execution::{HOST_OPERATION_PLAN_SCHEMA, HostOperationPlan, admit_host_operation_plan};
use crate::ports::HostCommandRunnerPort;

use super::{
    AgentAuthorityProjectionObservation, AgentAuthorityProjectionPlan,
    AgentAuthorityProjectionSpec, observe_agent_authority_projection,
    plan_agent_authority_projection,
};

const POLICY_ID: &str = "cynic.host.agent_authority.install";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAuthorityCommandExecution {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAuthorityApplyReport {
    pub schema: &'static str,
    pub status: String,
    pub requested_apply: bool,
    pub plan: AgentAuthorityProjectionPlan,
    pub before: AgentAuthorityProjectionObservation,
    pub executions: Vec<AgentAuthorityCommandExecution>,
    pub after: AgentAuthorityProjectionObservation,
    pub blockers: Vec<String>,
}

pub fn apply_agent_authority_projection(
    spec: &AgentAuthorityProjectionSpec,
    actor: &str,
    explicit_apply: bool,
    runner: &dyn HostCommandRunnerPort,
) -> AgentAuthorityApplyReport {
    let source = observe_agent_authority_projection(&spec.source_path);
    let before = observe_agent_authority_projection(&spec.target_path);
    let plan = plan_agent_authority_projection(spec, &source, &before);
    if plan.status == "blocked" || plan.status == "converged" || !explicit_apply {
        return report_without_execution(plan, before, explicit_apply);
    }

    let operation_plan = operation_plan(spec, &plan);
    let admission = admission(&operation_plan, actor);
    let authorization = ExecutionAuthorization::new(
        actor,
        PolicyAuthority::HumanAdmin,
        true,
        operation_plan.preconditions.clone(),
    );
    let admitted = match admit_host_operation_plan(operation_plan, admission, authorization) {
        Ok(admitted) => admitted,
        Err(blockers) => {
            return AgentAuthorityApplyReport {
                schema: "cynic.host.agent_authority_apply_report.v1",
                status: "blocked".to_string(),
                requested_apply: true,
                plan,
                before: before.clone(),
                executions: Vec::new(),
                after: before,
                blockers,
            };
        }
    };
    let mut executions = Vec::new();
    for operation in admitted.operations() {
        let result = runner.run_operation(operation);
        let success = result.success;
        executions.push(AgentAuthorityCommandExecution {
            command: result.command,
            success,
            exit_code: result.exit_code,
            stderr: result.stderr,
        });
        if !success {
            break;
        }
    }
    let after = observe_agent_authority_projection(&spec.target_path);
    let read_back = plan_agent_authority_projection(spec, &source, &after);
    let mut blockers = Vec::new();
    if executions.len() != admitted.operations().len()
        || executions.iter().any(|execution| !execution.success)
    {
        blockers.push("agent_authority_install_execution_failed".to_string());
    }
    if read_back.status != "converged" {
        blockers.push("agent_authority_install_read_back_failed".to_string());
    }
    AgentAuthorityApplyReport {
        schema: "cynic.host.agent_authority_apply_report.v1",
        status: if blockers.is_empty() {
            "completed"
        } else {
            "blocked"
        }
        .to_string(),
        requested_apply: true,
        plan,
        before,
        executions,
        after,
        blockers,
    }
}

fn report_without_execution(
    plan: AgentAuthorityProjectionPlan,
    before: AgentAuthorityProjectionObservation,
    requested_apply: bool,
) -> AgentAuthorityApplyReport {
    let status = match plan.status.as_str() {
        "converged" => "completed",
        "blocked" => "blocked",
        _ => "planned",
    };
    let blockers = if !requested_apply && plan.status == "planned" {
        vec!["explicit_apply_required".to_string()]
    } else {
        plan.blockers.clone()
    };
    AgentAuthorityApplyReport {
        schema: "cynic.host.agent_authority_apply_report.v1",
        status: status.to_string(),
        requested_apply,
        plan,
        before: before.clone(),
        executions: Vec::new(),
        after: before,
        blockers,
    }
}

fn operation_plan(
    spec: &AgentAuthorityProjectionSpec,
    plan: &AgentAuthorityProjectionPlan,
) -> HostOperationPlan {
    HostOperationPlan {
        schema: HOST_OPERATION_PLAN_SCHEMA,
        id: format!("agent-authority-install:{}", plan.desired_authority_digest),
        kind: "install_agent_session_authority".to_string(),
        domain: "agent_access".to_string(),
        target: spec.target_path.clone(),
        mutates: true,
        requires_privilege: true,
        operations: plan.operations.clone(),
        preconditions: vec![
            "authority_source_digest_proven".to_string(),
            "authority_target_path_declared".to_string(),
        ],
        rollback_intent: plan.rollback_intent.clone(),
        expected_proofs: plan
            .expected_proofs
            .iter()
            .map(|gate| RequiredProof {
                gate: gate.clone(),
                subject: spec.target_path.clone(),
            })
            .collect(),
    }
}

fn admission(plan: &HostOperationPlan, actor: &str) -> PolicyAdmissionReport {
    let plan_digest = digest_json(plan).unwrap_or_else(|_| "unavailable".to_string());
    let mutation = MutationContract {
        kind: plan.kind.clone(),
        scope: PolicyScope {
            domain: plan.domain.clone(),
            target: plan.target.clone(),
        },
        mutates: true,
        requires_privilege: true,
        requires_explicit_apply: true,
        required_authority: PolicyAuthority::HumanAdmin,
        preconditions: plan.preconditions.clone(),
        rollback_intent: plan.rollback_intent.clone(),
        expected_proofs: plan.expected_proofs.clone(),
    };
    PolicyAdmissionReport::new(
        PolicySetRef {
            id: POLICY_ID.to_string(),
            version: "v1".to_string(),
            digest: digest_json(&serde_json::json!({
                "id": POLICY_ID,
                "authority": "human_admin",
                "target": plan.target,
            }))
            .unwrap_or_else(|_| "unavailable".to_string()),
        },
        PolicyIntentRef {
            id: plan.id.clone(),
            kind: plan.kind.clone(),
            actor: actor.to_string(),
            subject: plan.target.clone(),
            digest: plan_digest.clone(),
            plan_digest,
        },
        mutation.clone(),
        PolicyDecision::Allow {
            required_authority: PolicyAuthority::HumanAdmin,
            required_proofs: mutation.expected_proofs.clone(),
        },
    )
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    use cynic_protocol::AgentCapabilityGrant;

    use super::*;
    use crate::agent_access::{AgentEnrollmentPolicy, compose_agent_session_authority};
    use crate::execution::AdmittedHostOperation;
    use crate::ports::{HostCommandResult, HostOperation};

    struct CopyRunner;

    impl HostCommandRunnerPort for CopyRunner {
        fn run_operation(&self, admitted: &AdmittedHostOperation) -> HostCommandResult {
            let HostOperation::InstallOwnedFile {
                source,
                target,
                mode,
                ..
            } = admitted.operation()
            else {
                panic!("unexpected operation");
            };
            let result = std::fs::copy(source, target).and_then(|_| {
                std::fs::set_permissions(target, std::fs::Permissions::from_mode(*mode))
            });
            HostCommandResult {
                command: format!("install {source} {target}"),
                success: result.is_ok(),
                exit_code: Some(if result.is_ok() { 0 } else { 1 }),
                stdout: String::new(),
                stderr: result
                    .err()
                    .map(|error| error.to_string())
                    .unwrap_or_default(),
            }
        }
    }

    #[test]
    fn explicit_apply_requires_read_back_before_completion() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("authority.json");
        let target = directory.path().join("installed.json");
        let authority = compose_agent_session_authority(
            "node-a",
            1,
            2_000,
            &[AgentEnrollmentPolicy {
                observed_linux_uid: 1000,
                agent_types: vec!["codex".to_string()],
                max_ttl_ms: 900_000,
                capabilities: vec![AgentCapabilityGrant {
                    id: "coord.read".to_string(),
                    mutates: false,
                }],
            }],
        )
        .unwrap();
        std::fs::write(&source, serde_json::to_vec(&authority).unwrap()).unwrap();
        let metadata = std::fs::metadata(&source).unwrap();
        let mut spec =
            AgentAuthorityProjectionSpec::installed(source.display().to_string(), authority);
        spec.target_path = target.display().to_string();
        spec.owner_id = Some(metadata.uid());
        spec.group_id = Some(metadata.gid());

        let planned = apply_agent_authority_projection(&spec, "test-admin", false, &CopyRunner);
        assert_eq!(planned.status, "planned");
        assert_eq!(planned.blockers, vec!["explicit_apply_required"]);

        let applied = apply_agent_authority_projection(&spec, "test-admin", true, &CopyRunner);
        assert_eq!(applied.status, "completed");
        assert_eq!(applied.after.content_valid, Some(true));
    }
}
