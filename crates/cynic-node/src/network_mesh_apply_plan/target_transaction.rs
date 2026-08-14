use std::collections::BTreeSet;

use chrono::Utc;
use cynic_host::runtime_harness::RuntimeHarnessContext;
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::NetworkMeshApplyPlanReport;
use crate::domain::network_execution::{NetworkExecutionAdmission, admit_network_execution};
use crate::network_actuator::CommandExecution;
use crate::network_command_surface::{NetworkCommandInvocation, NetworkCommandSurface};
use crate::network_diagnostic_surface::{NetworkDiagnostic, diagnostic};
use crate::ports::network_command_runner_port::NetworkCommandRunnerPort;

const TRANSACTIONAL_PROFILE: &str = "mesh-route-install";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkMeshTargetTransactionPhase {
    pub(crate) commands: Vec<NetworkCommandSurface>,
    pub(crate) executions: Vec<CommandExecution>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkMeshTargetTransactionReport {
    pub(crate) schema: &'static str,
    pub(crate) generated_at: String,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) profile: String,
    pub(crate) apply: bool,
    pub(crate) bundle_dir: String,
    pub(crate) execution_admission: NetworkExecutionAdmission,
    pub(crate) before: NetworkMeshTargetTransactionPhase,
    pub(crate) apply_phase: NetworkMeshTargetTransactionPhase,
    pub(crate) after: NetworkMeshTargetTransactionPhase,
    pub(crate) rollback: NetworkMeshTargetTransactionPhase,
    pub(crate) rollback_verification: NetworkMeshTargetTransactionPhase,
    pub(crate) proof_refs: Vec<String>,
    pub(crate) violations: Vec<NetworkDiagnostic>,
}

pub(crate) fn target_transaction_argv(
    plan: &NetworkMeshApplyPlanReport,
    profile: &str,
) -> Vec<Vec<String>> {
    if profile != TRANSACTIONAL_PROFILE {
        return Vec::new();
    }
    plan.route_observation
        .iter()
        .chain(plan.route_install.iter())
        .chain(plan.rollback_plan.iter())
        .filter_map(argv)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn apply_network_mesh_target_transaction_with_runner(
    bundle_dir: &str,
    profile: &str,
    apply: bool,
    harness: &RuntimeHarnessContext,
    runner: &impl NetworkCommandRunnerPort,
    plan: NetworkMeshApplyPlanReport,
) -> NetworkMeshTargetTransactionReport {
    let execution_admission = admit_network_execution(
        profile,
        true,
        runner.execution_boundary(),
        Some(harness),
        runner.target_attestation(),
    );
    let before_commands = plan.route_observation.clone();
    let apply_commands = plan.route_install.clone();
    let rollback_commands = plan
        .rollback_plan
        .iter()
        .filter(|command| matches!(command.invocation, NetworkCommandInvocation::Argv { .. }))
        .cloned()
        .collect::<Vec<_>>();
    let mut report = NetworkMeshTargetTransactionReport {
        schema: "cynic.network_mesh_target_transaction.v1",
        generated_at: Utc::now().to_rfc3339(),
        status: "refused",
        mutates: false,
        profile: profile.to_string(),
        apply,
        bundle_dir: bundle_dir.to_string(),
        execution_admission,
        before: phase(before_commands.clone()),
        apply_phase: phase(apply_commands.clone()),
        after: phase(before_commands.clone()),
        rollback: phase(rollback_commands.clone()),
        rollback_verification: phase(before_commands),
        proof_refs: Vec::new(),
        violations: Vec::new(),
    };

    validate_transaction_request(
        &plan,
        profile,
        apply,
        &report.execution_admission,
        &apply_commands,
        &report.before.commands,
        &rollback_commands,
        &mut report.violations,
    );
    if !report.violations.is_empty() {
        return report;
    }

    report.before.executions = execute_until_failure(runner, &report.before.commands);
    add_phase_proof("before", &report.before.executions, &mut report.proof_refs);
    if !phase_succeeded(&report.before)
        || report
            .before
            .executions
            .iter()
            .any(|execution| !execution.stdout.trim().is_empty())
    {
        report.status = "preflight_refused";
        report.violations.push(diagnostic(
            "route_prestate_not_empty",
            "transaction_preflight",
            "transaction requires every planned route to be absent before apply",
        ));
        return report;
    }

    report.apply_phase.executions = execute_until_failure(runner, &report.apply_phase.commands);
    report.mutates = !report.apply_phase.executions.is_empty();
    add_phase_proof(
        "apply",
        &report.apply_phase.executions,
        &mut report.proof_refs,
    );
    let attempted = report.apply_phase.executions.len();
    if !phase_succeeded(&report.apply_phase) {
        rollback_attempted_routes(runner, attempted, &mut report);
        return report;
    }

    report.after.executions = execute_until_failure(runner, &report.after.commands);
    add_phase_proof("after", &report.after.executions, &mut report.proof_refs);
    if !phase_succeeded(&report.after)
        || !route_poststate_matches(&report.apply_phase.commands, &report.after.executions)
    {
        report.violations.push(diagnostic(
            "route_poststate_unverified",
            "transaction_observation",
            "post-apply observation did not prove every planned route on the mesh interface",
        ));
        rollback_attempted_routes(runner, attempted, &mut report);
        return report;
    }

    report.status = "completed";
    report
}

fn validate_transaction_request(
    plan: &NetworkMeshApplyPlanReport,
    profile: &str,
    apply: bool,
    admission: &NetworkExecutionAdmission,
    apply_commands: &[NetworkCommandSurface],
    observation_commands: &[NetworkCommandSurface],
    rollback_commands: &[NetworkCommandSurface],
    violations: &mut Vec<NetworkDiagnostic>,
) {
    if profile != TRANSACTIONAL_PROFILE {
        violations.push(diagnostic(
            "profile_not_transactional",
            "transaction_policy",
            "target transaction currently supports only mesh-route-install",
        ));
    }
    if !apply {
        violations.push(diagnostic(
            "apply_flag_required",
            "request_validation",
            "apply flag is required for target transaction execution",
        ));
    }
    if !plan.bundle_complete {
        violations.push(diagnostic(
            "bundle_incomplete",
            "request_validation",
            "mesh bundle is incomplete",
        ));
    }
    if apply_commands.is_empty()
        || apply_commands.len() != observation_commands.len()
        || apply_commands.len() != rollback_commands.len()
        || apply_commands.iter().any(|command| argv(command).is_none())
        || observation_commands
            .iter()
            .any(|command| argv(command).is_none())
    {
        violations.push(diagnostic(
            "transaction_command_mapping_invalid",
            "transaction_policy",
            "apply, observation, and rollback argv must form a complete one-to-one mapping",
        ));
    }
    violations.extend(
        admission
            .violations
            .iter()
            .map(|violation| diagnostic(violation.code, "execution_admission", violation.message)),
    );
}

fn rollback_attempted_routes(
    runner: &impl NetworkCommandRunnerPort,
    attempted: usize,
    report: &mut NetworkMeshTargetTransactionReport,
) {
    report.rollback.commands = report
        .rollback
        .commands
        .iter()
        .take(attempted)
        .cloned()
        .rev()
        .collect();
    report.rollback.executions = execute_until_failure(runner, &report.rollback.commands);
    add_phase_proof(
        "rollback",
        &report.rollback.executions,
        &mut report.proof_refs,
    );
    if !phase_succeeded(&report.rollback)
        || report.rollback.executions.len() != report.rollback.commands.len()
    {
        report.status = "rollback_failed";
        report.violations.push(diagnostic(
            "route_rollback_failed",
            "transaction_rollback",
            "bounded route rollback did not complete",
        ));
        return;
    }

    report.rollback_verification.executions =
        execute_until_failure(runner, &report.rollback_verification.commands);
    add_phase_proof(
        "rollback_verification",
        &report.rollback_verification.executions,
        &mut report.proof_refs,
    );
    if phase_succeeded(&report.rollback_verification)
        && report
            .rollback_verification
            .executions
            .iter()
            .all(|execution| execution.stdout.trim().is_empty())
    {
        report.status = "rolled_back";
    } else {
        report.status = "rollback_failed";
        report.violations.push(diagnostic(
            "route_rollback_unverified",
            "transaction_rollback",
            "post-rollback observation did not prove the original empty route state",
        ));
    }
}

fn execute_until_failure(
    runner: &impl NetworkCommandRunnerPort,
    commands: &[NetworkCommandSurface],
) -> Vec<CommandExecution> {
    let mut executions = Vec::new();
    for command in commands {
        let execution = runner.run_command(command);
        let success = execution.exit_code == Some(0);
        executions.push(execution);
        if !success {
            break;
        }
    }
    executions
}

fn route_poststate_matches(
    commands: &[NetworkCommandSurface],
    observations: &[CommandExecution],
) -> bool {
    commands.len() == observations.len()
        && commands
            .iter()
            .zip(observations)
            .all(|(command, observation)| {
                let Some(argv) = argv(command) else {
                    return false;
                };
                matches!(
                    argv.as_slice(),
                    [_, route, replace, destination, dev, interface]
                        if route == "route" && replace == "replace" && dev == "dev"
                            && observation.stdout.contains(destination)
                            && observation.stdout.contains(interface)
                )
            })
}

fn phase(commands: Vec<NetworkCommandSurface>) -> NetworkMeshTargetTransactionPhase {
    NetworkMeshTargetTransactionPhase {
        commands,
        executions: Vec::new(),
    }
}

fn phase_succeeded(phase: &NetworkMeshTargetTransactionPhase) -> bool {
    phase.executions.len() == phase.commands.len()
        && phase
            .executions
            .iter()
            .all(|execution| execution.exit_code == Some(0))
}

fn argv(command: &NetworkCommandSurface) -> Option<Vec<String>> {
    match &command.invocation {
        NetworkCommandInvocation::Argv { argv } => Some(argv.clone()),
        _ => None,
    }
}

fn add_phase_proof(stage: &str, executions: &[CommandExecution], proof_refs: &mut Vec<String>) {
    if executions.is_empty() {
        return;
    }
    let mut digest = Sha256::new();
    digest.update(stage.as_bytes());
    for execution in executions {
        digest_field(&mut digest, execution.role.as_bytes());
        for argument in &execution.argv {
            digest_field(&mut digest, argument.as_bytes());
        }
        let exit_code = execution
            .exit_code
            .map_or_else(|| "none".to_string(), |code| code.to_string());
        digest_field(&mut digest, exit_code.as_bytes());
        digest_field(&mut digest, execution.stdout.as_bytes());
        digest_field(&mut digest, execution.stderr.as_bytes());
    }
    proof_refs.push(format!("sha256:{stage}:{}", hex::encode(digest.finalize())));
}

fn digest_field(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}
