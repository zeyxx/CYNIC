use chrono::Utc;
use cynic_host::runtime_harness::RuntimeHarnessContext;
use serde::Serialize;

use super::NetworkMeshApplyPlanReport;
use super::policy::MeshActuatorProfile;
use crate::adapters::local_network_command_runner::LocalNetworkCommandRunner;
use crate::domain::network_execution::{NetworkExecutionAdmission, admit_network_execution};
use crate::network_actuator::CommandExecution;
use crate::network_command_surface::{NetworkCommandInvocation, NetworkCommandSurface};
use crate::network_diagnostic_surface::{NetworkDiagnostic, diagnostic};
use crate::ports::network_command_runner_port::NetworkCommandRunnerPort;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkMeshApplyProfileReport {
    pub(crate) schema: &'static str,
    pub(crate) generated_at: String,
    pub(crate) status: &'static str,
    pub(crate) profile: String,
    pub(crate) apply: bool,
    pub(crate) closed_environment: bool,
    pub(crate) execution_admission: NetworkExecutionAdmission,
    pub(crate) bundle_dir: String,
    pub(crate) commands: Vec<NetworkCommandSurface>,
    pub(crate) executions: Vec<CommandExecution>,
    pub(crate) violations: Vec<NetworkDiagnostic>,
}

pub(super) fn apply_network_mesh_profile_from_plan(
    bundle_dir: &str,
    profile: &str,
    apply: bool,
    closed_environment: bool,
    plan: NetworkMeshApplyPlanReport,
) -> NetworkMeshApplyProfileReport {
    let harness = closed_environment.then(cynic_host::runtime_harness::closed_local_lab_context);
    apply_network_mesh_profile_from_plan_with_runner(
        bundle_dir,
        profile,
        apply,
        closed_environment,
        harness.as_ref(),
        &LocalNetworkCommandRunner,
        plan,
    )
}

pub(crate) fn apply_network_mesh_profile_from_plan_with_runner(
    bundle_dir: &str,
    profile: &str,
    apply: bool,
    closed_environment: bool,
    harness: Option<&RuntimeHarnessContext>,
    runner: &impl NetworkCommandRunnerPort,
    plan: NetworkMeshApplyPlanReport,
) -> NetworkMeshApplyProfileReport {
    let selected_profile = plan
        .actuator_policy
        .allowed_profiles
        .iter()
        .find(|item| item.name == profile);
    let mutates_network = is_mutating_mesh_profile(profile);
    let execution_admission = admit_network_execution(
        profile,
        mutates_network,
        runner.execution_boundary(),
        harness,
        runner.target_attestation(),
    );
    let mut violations =
        validate_mesh_profile_request(&plan, selected_profile, profile, apply, closed_environment);
    violations.extend(
        execution_admission
            .violations
            .iter()
            .map(|violation| diagnostic(violation.code, "execution_admission", violation.message)),
    );
    let commands = selected_profile
        .map(|item| item.command_prefixes.clone())
        .unwrap_or_default();

    if violations.is_empty() {
        violations.extend(non_executable_violations(&commands));
    }

    if !violations.is_empty() || commands.is_empty() {
        return NetworkMeshApplyProfileReport {
            schema: "cynic.network_mesh_apply_profile.v2",
            generated_at: Utc::now().to_rfc3339(),
            status: "refused",
            profile: profile.to_string(),
            apply,
            closed_environment,
            execution_admission,
            bundle_dir: bundle_dir.to_string(),
            commands,
            executions: Vec::new(),
            violations,
        };
    }

    let mut executions = Vec::new();
    for command in &commands {
        let execution = runner.run_command(command);
        let success = execution.exit_code == Some(0);
        executions.push(execution);
        if !success {
            break;
        }
    }
    let success = executions
        .iter()
        .all(|execution| execution.exit_code == Some(0));

    NetworkMeshApplyProfileReport {
        schema: "cynic.network_mesh_apply_profile.v2",
        generated_at: Utc::now().to_rfc3339(),
        status: if success { "completed" } else { "failed" },
        profile: profile.to_string(),
        apply,
        closed_environment,
        execution_admission,
        bundle_dir: bundle_dir.to_string(),
        commands,
        executions,
        violations: Vec::new(),
    }
}

fn validate_mesh_profile_request(
    plan: &NetworkMeshApplyPlanReport,
    selected_profile: Option<&MeshActuatorProfile>,
    profile: &str,
    apply: bool,
    closed_environment: bool,
) -> Vec<NetworkDiagnostic> {
    let mut violations = Vec::new();
    if profile.trim().is_empty() {
        violations.push(diagnostic(
            "missing_profile",
            "request_validation",
            "profile is required",
        ));
    }
    if !apply {
        violations.push(diagnostic(
            "apply_flag_required",
            "request_validation",
            "apply flag is required for mesh actuator execution",
        ));
    }
    if !plan.bundle_complete {
        violations.push(diagnostic(
            "bundle_incomplete",
            "request_validation",
            "mesh bundle is incomplete",
        ));
    }
    if selected_profile.is_none() {
        violations.push(diagnostic(
            "profile_not_allowed_for_mesh_bundle",
            "request_validation",
            "profile is not present in actuator_policy.allowed_profiles",
        ));
    }
    if is_mutating_mesh_profile(profile) && !closed_environment {
        violations.push(diagnostic(
            "closed_environment_required",
            "request_validation",
            "mutating mesh profiles require an explicitly declared closed environment",
        ));
    }
    violations
}

fn is_mutating_mesh_profile(profile: &str) -> bool {
    matches!(
        profile,
        "mesh-route-install" | "mesh-rollback" | "mesh-tunnel-install" | "mesh-dns-install"
    )
}

fn non_executable_violations(commands: &[NetworkCommandSurface]) -> Vec<NetworkDiagnostic> {
    commands
        .iter()
        .filter(|command| !matches!(command.invocation, NetworkCommandInvocation::Argv { .. }))
        .map(|command| {
            let _role = &command.role;
            diagnostic(
                "profile_contains_non_executable_surface",
                "request_validation",
                "profile contains non-argv command; mesh apply-profile only executes argv surfaces",
            )
        })
        .collect()
}
