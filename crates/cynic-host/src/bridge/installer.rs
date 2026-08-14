use std::path::Path;

use crate::command_surface::{HostCommandInvocation, HostCommandSurface};
use crate::execution::{
    HostExecutionPlan, HostExecutionStep, execute_execution_plan, host_execution_plan_from_commands,
};

use super::types::{HostExecBridgeCommandExecution, HostExecBridgeInstallReport};

pub(super) fn bridge_install_report_from_commands(
    bundle_dir: &Path,
    apply: bool,
    bundle_expected_complete: bool,
    install_commands: Vec<HostCommandSurface>,
    requires_privilege: bool,
) -> HostExecBridgeInstallReport {
    let bundle_dir_str = bundle_dir.to_string_lossy().to_string();
    let mut blockers = Vec::new();
    let execution_plan = bridge_install_execution_plan(install_commands, requires_privilege);

    if !bundle_expected_complete {
        blockers.push("bridge_bundle_incomplete".to_string());
    }

    let execution_report = if blockers.is_empty() {
        execute_execution_plan(&execution_plan, apply)
    } else {
        execute_execution_plan(
            &host_execution_plan_from_commands(
                "cynic.host_exec_bridge.install.blocked",
                "host_exec_bridge",
                Vec::new(),
                true,
                requires_privilege,
            ),
            true,
        )
    };
    blockers.extend(execution_report.blockers);
    let executions = execution_report
        .executions
        .into_iter()
        .map(|execution| HostExecBridgeCommandExecution {
            role: execution.role,
            argv: execution.argv,
            exit_code: execution.exit_code,
            stdout: execution.stdout,
            stderr: execution.stderr,
        })
        .collect::<Vec<_>>();

    let status = if !blockers.is_empty() {
        "blocked"
    } else if execution_report.status == "completed" {
        "completed"
    } else {
        "failed"
    };

    HostExecBridgeInstallReport {
        schema: "cynic.host_exec_bridge.install_report.v1",
        status,
        mutates: true,
        requested_apply: apply,
        bundle_dir: bundle_dir_str,
        bundle_expected_complete,
        execution_plan,
        executions,
        blockers,
    }
}

pub(super) fn bridge_install_execution_plan(
    install_commands: Vec<HostCommandSurface>,
    requires_privilege: bool,
) -> HostExecutionPlan {
    let mut plan = host_execution_plan_from_commands(
        "cynic.host_exec_bridge.install",
        "host_exec_bridge",
        install_commands,
        true,
        requires_privilege,
    );

    for step in &mut plan.steps {
        annotate_bridge_install_step(step);
    }

    plan
}

fn annotate_bridge_install_step(step: &mut HostExecutionStep) {
    match &step.command.invocation {
        HostCommandInvocation::InstallDir { path, .. } => {
            step.proof_paths.push(path.clone());
            step.rollback_hint = Some("remove directory only if empty and not shared".to_string());
        }
        HostCommandInvocation::InstallFile { target, .. } => {
            step.proof_paths.push(target.clone());
            step.rollback_hint = Some(format!("remove {target} after disabling dependent access"));
        }
        HostCommandInvocation::ValidateSudoers { path } => {
            step.preconditions.push(format!("{path} exists"));
            step.proof_paths.push(path.clone());
            step.rollback_hint = Some(format!("remove {path} if validation or policy check fails"));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::bridge_install_execution_plan;
    use crate::command_surface::validate_sudoers_command;

    #[test]
    fn bridge_install_execution_plan_records_privilege_and_proof_paths() {
        let commands = vec![validate_sudoers_command(
            "validate_sudoers",
            "/tmp/cynic-test.sudoers",
        )];

        let plan = bridge_install_execution_plan(commands, false);

        assert_eq!(plan.schema, "cynic.host.execution_plan.v1");
        assert_eq!(plan.id, "cynic.host_exec_bridge.install");
        assert_eq!(plan.layer, "host_exec_bridge");
        assert!(!plan.requires_privilege);
        assert_eq!(plan.steps[0].proof_paths, vec!["/tmp/cynic-test.sudoers"]);
        assert!(
            plan.steps[0]
                .preconditions
                .contains(&"/tmp/cynic-test.sudoers exists".to_string())
        );
    }
}
