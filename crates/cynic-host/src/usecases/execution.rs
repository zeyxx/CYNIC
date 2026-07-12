use crate::command_surface::HostCommandSurface;
use crate::domain::shared::SurfaceStatus;
use crate::ports::{HostCommandResult, HostCommandRunnerPort, HostContractPort, HostOperation};

use super::reports::{
    ExecutedHostCommand, ExecutedHostCommandResult, HOST_APPLY_EXECUTION_SCHEMA,
    HostApplyExecutionReport, rendered_dry_run_command,
};
use super::{HostObservationReports, build_host_apply_report};

pub fn execute_host_apply(
    port: &impl HostContractPort,
    observed: &HostObservationReports,
    dry_run: bool,
    runner: &impl HostCommandRunnerPort,
) -> HostApplyExecutionReport {
    let apply = build_host_apply_report(port, observed, dry_run);
    let mut commands = Vec::new();
    extend_commands(
        &mut commands,
        "principal",
        &apply.principal.commands,
        &port.desired_principal().apply_operations(),
        dry_run,
        runner,
    );
    extend_commands(
        &mut commands,
        "privilege",
        &apply.privilege.commands,
        &port.desired_privilege().apply_operations(),
        dry_run,
        runner,
    );
    extend_commands(
        &mut commands,
        "packages",
        &apply.packages.commands,
        &port.desired_packages().apply_operations(),
        dry_run,
        runner,
    );
    extend_commands(
        &mut commands,
        "services",
        &apply.services.commands,
        &port.desired_services().apply_operations(),
        dry_run,
        runner,
    );

    let executed_failure = commands
        .iter()
        .any(|item| item.executed && item.result.as_ref().is_some_and(|result| !result.success));

    HostApplyExecutionReport {
        schema: HOST_APPLY_EXECUTION_SCHEMA,
        host_resolution: apply.host_resolution.clone(),
        status: if dry_run {
            apply.status
        } else if executed_failure {
            SurfaceStatus::Blocked
        } else {
            SurfaceStatus::Active
        },
        dry_run,
        blocked_domains: apply.blocked_domains,
        next_actions: apply.next_actions,
        commands,
    }
}

fn extend_commands(
    target: &mut Vec<ExecutedHostCommand>,
    domain: &'static str,
    commands: &[HostCommandSurface],
    operations: &[HostOperation],
    dry_run: bool,
    runner: &impl HostCommandRunnerPort,
) {
    for (command, operation) in commands.iter().zip(operations.iter()) {
        if dry_run {
            target.push(rendered_dry_run_command(domain, command));
            continue;
        }

        let result = runner.run_operation(operation);
        target.push(ExecutedHostCommand {
            domain,
            command: result.command.clone(),
            executed: true,
            result: Some(executed_result(result)),
        });
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
