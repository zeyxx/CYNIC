use cynic_protocol::{ExecutionAuthorization, PolicyAdmissionReport, RequiredProof};
use serde::Serialize;

use crate::execution::{
    HostOperationPlan, admit_host_operation_plan, build_privilege_bridge_operation_plan,
    privilege_bridge_policy_admission,
};
use crate::ports::{
    HostCommandResult, HostCommandRunnerPort, HostOperation, HostPrivilegeBridgeObservationPort,
};

use super::{PrivilegeBridgeInstallSpec, PrivilegeBridgeObservedState};

pub const PRIVILEGE_BRIDGE_EXECUTION_SCHEMA: &str = "cynic.host.privilege_bridge_execution.v1";

#[derive(Debug, Clone, Serialize)]
pub struct PrivilegeBridgeExecutionReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub target: String,
    pub provider: String,
    pub operation_plan: HostOperationPlan,
    pub admission: PolicyAdmissionReport,
    pub operations: Vec<PrivilegeBridgeOperationReport>,
    pub blockers: Vec<String>,
    pub proofs_required: Vec<RequiredProof>,
    pub proofs_observed: Vec<RequiredProof>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrivilegeBridgeOperationReport {
    pub operation: HostOperation,
    pub executed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<PrivilegeBridgeOperationResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrivilegeBridgeOperationResult {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn privilege_bridge_plan(spec: &PrivilegeBridgeInstallSpec) -> HostOperationPlan {
    build_privilege_bridge_operation_plan(spec)
}

pub fn privilege_bridge_apply(
    spec: &PrivilegeBridgeInstallSpec,
    authorization: ExecutionAuthorization,
    runner: &impl HostCommandRunnerPort,
    observer: &impl HostPrivilegeBridgeObservationPort,
) -> PrivilegeBridgeExecutionReport {
    let plan = build_privilege_bridge_operation_plan(spec);
    let admission = privilege_bridge_policy_admission(&plan, authorization.actor.clone());
    let operations = plan
        .operations
        .iter()
        .cloned()
        .map(|operation| PrivilegeBridgeOperationReport {
            operation,
            executed: false,
            result: None,
        })
        .collect();
    let mut report = PrivilegeBridgeExecutionReport {
        schema: PRIVILEGE_BRIDGE_EXECUTION_SCHEMA,
        status: "blocked",
        mutates: true,
        requested_apply: authorization.explicit_apply,
        target: plan.target.clone(),
        provider: spec.provider.clone(),
        operation_plan: plan.clone(),
        admission: admission.clone(),
        operations,
        blockers: Vec::new(),
        proofs_required: plan.expected_proofs.clone(),
        proofs_observed: Vec::new(),
    };

    let admitted = match admit_host_operation_plan(plan, admission, authorization) {
        Ok(admitted) => admitted,
        Err(blockers) => {
            report.blockers = blockers;
            return report;
        }
    };

    let preflight = observer.observe_privilege_bridge(spec);
    preflight_blockers(spec, &preflight, &mut report.blockers);
    if !report.blockers.is_empty() {
        return report;
    }

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
                .push("privilege_bridge_operation_failed".to_string());
            return report;
        }
    }

    let observed = observer.observe_privilege_bridge(spec);
    report.proofs_observed = observed_proofs(&observed, &report.target);
    for required in &report.proofs_required {
        if !report.proofs_observed.contains(required) {
            report
                .blockers
                .push(format!("postflight_proof_missing:{}", required.gate));
        }
    }
    if report.blockers.is_empty() {
        report.status = "completed";
    }
    report
}

fn preflight_blockers(
    spec: &PrivilegeBridgeInstallSpec,
    observed: &PrivilegeBridgeObservedState,
    blockers: &mut Vec<String>,
) {
    if !observed.subject_present {
        blockers.push("runtime_principal_missing".to_string());
    }
    if !spec.bundle_complete {
        blockers.push("bridge_bundle_incomplete".to_string());
    }
    if !observed.provider_available {
        blockers.push(format!(
            "privilege_projection_provider_unavailable:{}",
            spec.provider
        ));
    }
}

fn observed_proofs(observed: &PrivilegeBridgeObservedState, target: &str) -> Vec<RequiredProof> {
    let checks = [
        ("helper_present", observed.helper_present),
        ("helper_executable", observed.helper_executable),
        ("helper_mode_matches", observed.helper_mode_matches),
        ("helper_authority_owned", observed.helper_authority_owned),
        ("projection_present", observed.projection_present),
        ("projection_mode_matches", observed.projection_mode_matches),
        (
            "projection_authority_owned",
            observed.projection_authority_owned,
        ),
        ("projection_valid", observed.projection_valid),
        (
            "invocation_policy_bounded",
            observed.invocation_policy_bounded,
        ),
        (
            "bounded_helper_noninteractive_and_shell_denied",
            observed.bounded_helper_noninteractive && observed.unrestricted_shell_denied,
        ),
    ];
    checks
        .into_iter()
        .filter(|(_, ready)| *ready)
        .map(|(gate, _)| RequiredProof {
            gate: gate.to_string(),
            subject: target.to_string(),
        })
        .collect()
}

fn operation_result(result: HostCommandResult) -> PrivilegeBridgeOperationResult {
    PrivilegeBridgeOperationResult {
        command: result.command,
        success: result.success,
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
    }
}
