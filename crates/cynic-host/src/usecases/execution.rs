use cynic_protocol::{ExecutionAuthorization, PolicyAdmissionReport, RequiredProof};

use crate::command_surface::HostCommandSurface;
use crate::domain::services::{ServiceObservedState, ServicePolicy};
use crate::domain::shared::SurfaceStatus;
use crate::execution::{
    admit_host_operation_plan, build_service_operation_plan,
    runtime_service_operation_policy_admission, service_operation_policy_admission,
};
use crate::ports::{
    HostCommandResult, HostCommandRunnerPort, HostContractPort, HostOperation,
    HostServiceObservationPort,
};

use super::reports::{
    ExecutedHostCommand, ExecutedHostCommandResult, HOST_APPLY_EXECUTION_SCHEMA,
    HostApplyExecutionReport, rendered_dry_run_command,
};
use super::{HostObservationReports, build_host_apply_report};

pub fn execute_host_apply(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
    _runner: &impl HostCommandRunnerPort,
) -> HostApplyExecutionReport {
    let apply = build_host_apply_report(port, observed, dry_run);
    let commands = all_dry_run_commands(&apply);
    let blockers = if dry_run {
        Vec::new()
    } else {
        vec!["policy_admission_required".to_string()]
    };

    HostApplyExecutionReport {
        schema: HOST_APPLY_EXECUTION_SCHEMA,
        host_resolution: apply.host_resolution,
        status: if dry_run {
            apply.status
        } else {
            SurfaceStatus::Blocked
        },
        dry_run,
        lifecycle_status: apply.lifecycle_status,
        lifecycle_disposition: apply.lifecycle_disposition,
        next_boundary_kind: apply.next_boundary_kind,
        next_boundary_reason: apply.next_boundary_reason,
        next_command: apply.next_command,
        blocked_domains: apply.blocked_domains,
        next_actions: apply.next_actions,
        admission: None,
        blockers,
        proofs_required: Vec::new(),
        proofs_observed: Vec::new(),
        commands,
    }
}

pub fn execute_admitted_service_apply(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    authorization: ExecutionAuthorization,
    runner: &impl HostCommandRunnerPort,
    observer: &impl HostServiceObservationPort,
) -> HostApplyExecutionReport {
    let policy = port.desired_services();
    let plan = build_service_operation_plan(&policy, execution_target(observed));
    let admission = service_operation_policy_admission(&plan, authorization.actor.clone());
    execute_service_apply_with_admission(
        observed,
        &policy,
        plan,
        admission,
        authorization,
        runner,
        observer,
    )
}

pub fn execute_runtime_service_reconcile(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    authorization: ExecutionAuthorization,
    runner: &impl HostCommandRunnerPort,
    observer: &impl HostServiceObservationPort,
) -> HostApplyExecutionReport {
    let policy = port.desired_services();
    let plan = build_service_operation_plan(&policy, execution_target(observed));
    let admission = runtime_service_operation_policy_admission(&plan, authorization.actor.clone());
    execute_service_apply_with_admission(
        observed,
        &policy,
        plan,
        admission,
        authorization,
        runner,
        observer,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_service_apply_with_admission(
    observed: &HostObservationReports,
    policy: &ServicePolicy,
    plan: crate::execution::HostOperationPlan,
    admission: PolicyAdmissionReport,
    authorization: ExecutionAuthorization,
    runner: &impl HostCommandRunnerPort,
    observer: &impl HostServiceObservationPort,
) -> HostApplyExecutionReport {
    let commands = plan
        .operations
        .iter()
        .map(rendered_service_operation)
        .collect::<Vec<_>>();
    let mut report = HostApplyExecutionReport {
        schema: HOST_APPLY_EXECUTION_SCHEMA,
        host_resolution: observed.host_resolution.clone(),
        status: SurfaceStatus::Blocked,
        dry_run: false,
        lifecycle_status: observed.services.lifecycle_status,
        lifecycle_disposition: observed.services.lifecycle_disposition,
        next_boundary_kind: observed.services.next_boundary_kind,
        next_boundary_reason: observed.services.next_boundary_reason.clone(),
        next_command: observed.services.next_command.clone(),
        blocked_domains: vec!["services"],
        next_actions: vec!["repair_host_runtime_services".to_string()],
        admission: Some(admission.clone()),
        blockers: substrate_precondition_blockers(observed),
        proofs_required: plan.expected_proofs.clone(),
        proofs_observed: Vec::new(),
        commands,
    };

    let canonical_plan = build_service_operation_plan(policy, plan.target.clone());
    if plan != canonical_plan {
        report
            .blockers
            .push("plan_not_derived_from_desired_service_policy".to_string());
        return report;
    }

    let admitted = match admit_host_operation_plan(plan, admission, authorization) {
        Ok(admitted) => admitted,
        Err(blockers) => {
            report.blockers.extend(blockers);
            return report;
        }
    };
    if !report.blockers.is_empty() {
        return report;
    }

    for (command, operation) in report.commands.iter_mut().zip(admitted.operations().iter()) {
        let result = runner.run_operation(operation);
        command.command = result.command.clone();
        command.executed = true;
        let success = result.success;
        command.result = Some(executed_result(result));
        if !success {
            report.blockers.push("service_operation_failed".to_string());
            return report;
        }
    }

    let post_state = observer.observe_services(policy);
    report.proofs_observed = observed_service_proofs(policy, &post_state, &admitted.plan.target);
    for required in &report.proofs_required {
        if !report.proofs_observed.contains(required) {
            report
                .blockers
                .push(format!("postflight_proof_missing:{}", required.gate));
        }
    }
    if report.blockers.is_empty() && policy.status(&post_state).status == SurfaceStatus::Active {
        report.status = SurfaceStatus::Active;
        report.lifecycle_status = crate::domain::shared::LifecycleStatus::Ready;
        report.lifecycle_disposition =
            crate::domain::shared::LifecycleDisposition::ObservableCandidate;
        report.next_boundary_kind = crate::domain::shared::BoundaryKind::Observe;
        report.next_boundary_reason =
            "runtime services are active and postflight proofs are satisfied".to_string();
        report.next_command = None;
        report.blocked_domains.clear();
        report.next_actions.clear();
    }
    report
}

pub fn service_apply_satisfied_preconditions(observed: &HostObservationReports) -> Vec<String> {
    let mut satisfied = vec!["service_policy_declared".to_string()];
    if observed.principal.status == SurfaceStatus::Active {
        satisfied.push("host_principal_active".to_string());
    }
    if observed.privilege.status == SurfaceStatus::Active {
        satisfied.push("host_privilege_active".to_string());
    }
    if observed.packages.status == SurfaceStatus::Active {
        satisfied.push("host_packages_active".to_string());
    }
    satisfied
}

fn substrate_precondition_blockers(observed: &HostObservationReports) -> Vec<String> {
    [
        ("principal", observed.principal.status),
        ("privilege", observed.privilege.status),
        ("packages", observed.packages.status),
    ]
    .into_iter()
    .filter(|(_, status)| *status != SurfaceStatus::Active)
    .map(|(domain, _)| format!("substrate_precondition_blocked:{domain}"))
    .collect()
}

fn execution_target(observed: &HostObservationReports) -> String {
    observed
        .host_resolution
        .as_ref()
        .and_then(|resolution| resolution.matched_node.clone())
        .or_else(|| {
            observed
                .host_resolution
                .as_ref()
                .map(|resolution| resolution.hostname_observed.clone())
        })
        .unwrap_or_else(|| "local-host".to_string())
}

fn all_dry_run_commands(apply: &super::HostApplyReport) -> Vec<ExecutedHostCommand> {
    let mut commands = Vec::new();
    extend_dry_run(&mut commands, "principal", &apply.principal.commands);
    extend_dry_run(&mut commands, "privilege", &apply.privilege.commands);
    extend_dry_run(&mut commands, "packages", &apply.packages.commands);
    extend_dry_run(&mut commands, "services", &apply.services.commands);
    commands
}

fn extend_dry_run(
    target: &mut Vec<ExecutedHostCommand>,
    domain: &'static str,
    commands: &[HostCommandSurface],
) {
    target.extend(
        commands
            .iter()
            .map(|command| rendered_dry_run_command(domain, command)),
    );
}

fn observed_service_proofs(
    policy: &ServicePolicy,
    observed: &ServiceObservedState,
    target: &str,
) -> Vec<RequiredProof> {
    let mut proofs = Vec::new();
    for desired in &policy.units {
        if let Some(unit) = observed
            .units
            .iter()
            .find(|unit| unit.unit_name == desired.unit_name)
        {
            if !desired.enable_on_boot || unit.enabled {
                proofs.push(RequiredProof {
                    gate: format!("unit_enabled:{}", desired.unit_name),
                    subject: target.to_string(),
                });
            }
            if unit.active {
                proofs.push(RequiredProof {
                    gate: format!("unit_active:{}", desired.unit_name),
                    subject: target.to_string(),
                });
            }
        }
    }
    proofs
}

fn rendered_service_operation(operation: &HostOperation) -> ExecutedHostCommand {
    let command = match operation {
        HostOperation::EnsureDir { path, mode } => {
            format!("ensure_dir path={path} mode={mode:o}")
        }
        HostOperation::ServiceManagerDaemonReload => "service_manager daemon-reload".to_string(),
        HostOperation::ServiceEnableNow { unit } => format!("service enable-now {unit}"),
        HostOperation::ServiceStart { unit } => format!("service start {unit}"),
        HostOperation::WritePackageSourceProjection { .. }
        | HostOperation::InstallBootstrapPackages { .. }
        | HostOperation::EnsureSystemGroup { .. }
        | HostOperation::EnsureSystemUser { .. }
        | HostOperation::EnsureSupplementaryGroupMember { .. }
        | HostOperation::EnsureOwnedDir { .. }
        | HostOperation::InstallOwnedFile { .. }
        | HostOperation::ValidatePrivilegeProjection { .. } => {
            "non_service_operation_rejected".to_string()
        }
    };
    ExecutedHostCommand {
        domain: "services",
        command,
        executed: false,
        result: None,
    }
}

fn executed_result(result: HostCommandResult) -> ExecutedHostCommandResult {
    ExecutedHostCommandResult {
        success: result.success,
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
    }
}
