use cynic_protocol::{
    ExecutionAuthorization, MutationContract, PolicyAdmissionReport, PolicyAuthority,
    PolicyDecision, PolicyIntentRef, PolicyScope, PolicySetRef, RequiredProof, digest_json,
};
use serde::Serialize;

use crate::adapters::agent_access::observe_agent_access;
use crate::execution::{HOST_OPERATION_PLAN_SCHEMA, HostOperationPlan, admit_host_operation_plan};
use crate::ports::HostCommandRunnerPort;

use super::{AgentAccessObservation, AgentAccessPlan, AgentAccessPolicy, reconcile_agent_access};

const POLICY_ID: &str = "cynic.host.agent_access.apply";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAccessCommandExecution {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AgentAccessApplyReport {
    pub schema: &'static str,
    pub status: String,
    pub requested_apply: bool,
    pub plan: AgentAccessPlan,
    pub before: AgentAccessObservation,
    pub executions: Vec<AgentAccessCommandExecution>,
    pub after: AgentAccessObservation,
    pub blockers: Vec<String>,
}

pub fn apply_agent_access(
    policy: &AgentAccessPolicy,
    actor: &str,
    explicit_apply: bool,
    runner: &dyn HostCommandRunnerPort,
) -> AgentAccessApplyReport {
    let before = observe_agent_access(policy);
    let plan = reconcile_agent_access(policy, &before);
    if plan.status == "blocked" || plan.status == "converged" || !explicit_apply {
        return report_without_execution(plan, before, explicit_apply);
    }

    let operation_plan = operation_plan(policy, &plan);
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
            return AgentAccessApplyReport {
                schema: "cynic.host.agent_access_apply_report.v1",
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
        executions.push(AgentAccessCommandExecution {
            command: result.command,
            success,
            exit_code: result.exit_code,
            stderr: result.stderr,
        });
        if !success {
            break;
        }
    }
    let after = observe_agent_access(policy);
    let read_back = reconcile_agent_access(policy, &after);
    let mut blockers = Vec::new();
    if executions.len() != admitted.operations().len()
        || executions.iter().any(|execution| !execution.success)
    {
        blockers.push("agent_access_execution_failed".to_string());
    }
    if read_back.status != "converged" {
        blockers.push("agent_access_read_back_failed".to_string());
    }
    AgentAccessApplyReport {
        schema: "cynic.host.agent_access_apply_report.v1",
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
    plan: AgentAccessPlan,
    before: AgentAccessObservation,
    requested_apply: bool,
) -> AgentAccessApplyReport {
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
    AgentAccessApplyReport {
        schema: "cynic.host.agent_access_apply_report.v1",
        status: status.to_string(),
        requested_apply,
        plan,
        before: before.clone(),
        executions: Vec::new(),
        after: before,
        blockers,
    }
}

fn operation_plan(policy: &AgentAccessPolicy, plan: &AgentAccessPlan) -> HostOperationPlan {
    let policy_digest = digest_json(policy).unwrap_or_else(|_| "unavailable".to_string());
    HostOperationPlan {
        schema: HOST_OPERATION_PLAN_SCHEMA,
        id: format!("agent-access:{policy_digest}"),
        kind: "reconcile_agent_access".to_string(),
        domain: "agent_access".to_string(),
        target: policy.socket_path.clone(),
        mutates: true,
        requires_privilege: true,
        operations: plan.operations.clone(),
        preconditions: vec!["authorized_principals_declared".to_string()],
        rollback_intent: "remove only the newly declared supplementary memberships and access group after operator review".to_string(),
        expected_proofs: plan
            .expected_proofs
            .iter()
            .map(|gate| RequiredProof {
                gate: gate.clone(),
                subject: policy.socket_path.clone(),
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
