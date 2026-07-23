use cynic_protocol::{ExecutionAuthorization, PolicyAdmissionReport, RequiredProof};
use serde::Serialize;

use super::{PrincipalPreparePolicyView, operation_plan, policy_view};
use crate::domain::principal::{PrincipalObservedState, PrincipalPolicy};
use crate::domain::shared::SurfaceStatus;
use crate::execution::{
    HostOperationPlan, admit_host_operation_plan, build_principal_operation_plan,
    principal_operation_policy_admission,
};
use crate::ports::{
    HostCommandResult, HostCommandRunnerPort, HostOperation, HostPrincipalObservationPort,
};
use crate::registry::HostProfileResolution;

pub const PRINCIPAL_PREPARE_EXECUTION_SCHEMA: &str = "cynic.host.principal_prepare_execution.v1";

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalPrepareExecutionReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub target: String,
    pub policy: PrincipalPreparePolicyView,
    pub operation_plan: HostOperationPlan,
    pub admission: PolicyAdmissionReport,
    pub operations: Vec<PrincipalPrepareOperationReport>,
    pub blockers: Vec<String>,
    pub proofs_required: Vec<RequiredProof>,
    pub proofs_observed: Vec<RequiredProof>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalPrepareOperationReport {
    pub operation: HostOperation,
    pub executed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<PrincipalPrepareOperationResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalPrepareOperationResult {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn principal_prepare_apply(
    policy: &PrincipalPolicy,
    resolution: Option<&HostProfileResolution>,
    authorization: ExecutionAuthorization,
    runner: &impl HostCommandRunnerPort,
    observer: &impl HostPrincipalObservationPort,
) -> PrincipalPrepareExecutionReport {
    let plan = operation_plan(policy, resolution);
    let admission = principal_operation_policy_admission(&plan, authorization.actor.clone());
    execute_with_admission(policy, plan, admission, authorization, runner, observer)
}

pub fn principal_prepare_preconditions() -> Vec<String> {
    vec!["principal_policy_declared".to_string()]
}

fn execute_with_admission(
    policy: &PrincipalPolicy,
    plan: HostOperationPlan,
    admission: PolicyAdmissionReport,
    authorization: ExecutionAuthorization,
    runner: &impl HostCommandRunnerPort,
    observer: &impl HostPrincipalObservationPort,
) -> PrincipalPrepareExecutionReport {
    let operations = plan
        .operations
        .iter()
        .cloned()
        .map(|operation| PrincipalPrepareOperationReport {
            operation,
            executed: false,
            result: None,
        })
        .collect();
    let mut report = PrincipalPrepareExecutionReport {
        schema: PRINCIPAL_PREPARE_EXECUTION_SCHEMA,
        status: "blocked",
        mutates: true,
        requested_apply: authorization.explicit_apply,
        target: plan.target.clone(),
        policy: policy_view(policy),
        operation_plan: plan.clone(),
        admission: admission.clone(),
        operations,
        blockers: Vec::new(),
        proofs_required: plan.expected_proofs.clone(),
        proofs_observed: Vec::new(),
    };

    let canonical = build_principal_operation_plan(policy, plan.target.clone());
    if plan != canonical {
        report
            .blockers
            .push("plan_not_derived_from_desired_principal_policy".to_string());
        return report;
    }
    let admitted = match admit_host_operation_plan(plan, admission, authorization) {
        Ok(admitted) => admitted,
        Err(blockers) => {
            report.blockers = blockers;
            return report;
        }
    };

    for (operation_report, operation) in report
        .operations
        .iter_mut()
        .zip(admitted.operations().iter())
    {
        let result = runner.run_operation(operation);
        let success = result.success;
        operation_report.executed = true;
        operation_report.result = Some(operation_result(result));
        if !success {
            report.status = "failed";
            report
                .blockers
                .push("principal_operation_failed".to_string());
            return report;
        }
    }

    let observed = observer.observe_principal(policy);
    report.proofs_observed = observed_proofs(policy, &observed, &report.target);
    for required in &report.proofs_required {
        if !report.proofs_observed.contains(required) {
            report
                .blockers
                .push(format!("postflight_proof_missing:{}", required.gate));
        }
    }
    if report.blockers.is_empty() && policy.status(&observed).status == SurfaceStatus::Active {
        report.status = "completed";
    }
    report
}

fn observed_proofs(
    policy: &PrincipalPolicy,
    observed: &PrincipalObservedState,
    target: &str,
) -> Vec<RequiredProof> {
    let mut proofs = Vec::new();
    if observed.user_present {
        proofs.push(proof("principal_user_present", target));
    }
    if observed.group_present {
        proofs.push(proof("principal_group_present", target));
    }
    if observed.observed_home_dir.as_deref() == Some(policy.home_dir.as_str()) {
        proofs.push(proof("principal_home_matches", target));
    }
    proofs.extend(
        policy
            .state_dirs
            .iter()
            .filter(|path| observed.present_state_dirs.contains(path))
            .map(|path| proof(&format!("state_dir_present:{path}"), target)),
    );
    proofs.extend(
        policy
            .managed_paths()
            .iter()
            .filter(|path| observed.owned_paths.contains(path))
            .map(|path| proof(&format!("path_owned:{path}"), target)),
    );
    proofs
}

fn proof(gate: &str, target: &str) -> RequiredProof {
    RequiredProof {
        gate: gate.to_string(),
        subject: target.to_string(),
    }
}

fn operation_result(result: HostCommandResult) -> PrincipalPrepareOperationResult {
    PrincipalPrepareOperationResult {
        command: result.command,
        success: result.success,
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
    }
}
