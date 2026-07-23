use crate::command_surface::{HostCommandInvocation, HostCommandSurface};
use crate::process_observation::capture_argv;
use crate::runtime_layout::current_runtime_layout;

use super::types::{
    HostArgvCommand, HostCommandExecution, HostExecutionPlan, HostExecutionPlanReport,
    HostExecutionReport, HostExecutionStep, HostExecutionStepExecution,
};

pub fn execute_execution_plan(plan: &HostExecutionPlan, apply: bool) -> HostExecutionPlanReport {
    let mut blockers = Vec::new();
    let mut executions = Vec::new();

    if !apply {
        blockers.push("apply_flag_required".to_string());
    }
    if plan.mutates {
        blockers.push("policy_admission_required".to_string());
    }
    if !plan.steps.is_empty()
        && !blockers
            .iter()
            .any(|item| item == "policy_admission_required")
    {
        blockers.push("policy_admission_required".to_string());
    }

    if blockers.is_empty() {
        for step in &plan.steps {
            match command_to_argv(&step.command) {
                Ok(argv) => {
                    let execution = run_argv(step.role.clone(), argv);
                    let success = step_accepts_exit_code(step, execution.exit_code);
                    executions.push(HostExecutionStepExecution {
                        step_id: step.id.clone(),
                        role: execution.role,
                        argv: execution.argv,
                        exit_code: execution.exit_code,
                        success,
                        stdout: execution.stdout,
                        stderr: execution.stderr,
                        proof_paths: step.proof_paths.clone(),
                        rollback_hint: step.rollback_hint.clone(),
                    });
                    if !success {
                        break;
                    }
                }
                Err(error) => {
                    blockers.push(format!("{}: {}: {error}", step.id, step.role));
                    break;
                }
            }
        }
    }

    HostExecutionPlanReport {
        schema: "cynic.host.execution_plan_report.v1",
        status: step_report_status(&executions, plan.steps.len(), &blockers),
        plan_id: plan.id.clone(),
        layer: plan.layer.clone(),
        mutates: plan.mutates,
        requires_privilege: plan.requires_privilege,
        requested_apply: apply,
        step_count: plan.steps.len(),
        execution_count: executions.len(),
        executions,
        blockers,
    }
}

pub fn execute_host_commands(
    commands: &[HostCommandSurface],
    apply: bool,
    mutates: bool,
) -> HostExecutionReport {
    let mut blockers = Vec::new();
    let mut executions = Vec::new();

    if !apply {
        blockers.push("apply_flag_required".to_string());
    }
    if mutates {
        blockers.push("policy_admission_required".to_string());
    }
    if !commands.is_empty()
        && !blockers
            .iter()
            .any(|item| item == "policy_admission_required")
    {
        blockers.push("policy_admission_required".to_string());
    }

    if blockers.is_empty() {
        for command in commands {
            match command_to_argv(command) {
                Ok(argv) => {
                    let role = command.role.clone();
                    let execution = run_argv(role, argv);
                    let success = execution.exit_code == Some(0);
                    executions.push(execution);
                    if !success {
                        break;
                    }
                }
                Err(error) => {
                    blockers.push(format!("{}: {error}", command.role));
                    break;
                }
            }
        }
    }

    HostExecutionReport {
        schema: "cynic.host.execution_report.v1",
        status: report_status(&executions, commands.len(), &blockers),
        mutates,
        requested_apply: apply,
        execution_count: executions.len(),
        executions,
        blockers,
    }
}

pub fn execute_argv_commands(
    commands: &[HostArgvCommand],
    apply: bool,
    mutates: bool,
) -> HostExecutionReport {
    let mut blockers = Vec::new();
    let mut executions = Vec::new();

    if !apply {
        blockers.push("apply_flag_required".to_string());
    }
    if mutates {
        blockers.push("policy_admission_required".to_string());
    }
    if !commands.is_empty()
        && !blockers
            .iter()
            .any(|item| item == "policy_admission_required")
    {
        blockers.push("policy_admission_required".to_string());
    }

    if blockers.is_empty() {
        for command in commands {
            let execution = run_argv(command.role.clone(), command.argv.clone());
            let success = execution.exit_code == Some(0);
            executions.push(execution);
            if !success {
                break;
            }
        }
    }

    HostExecutionReport {
        schema: "cynic.host.execution_report.v1",
        status: report_status(&executions, commands.len(), &blockers),
        mutates,
        requested_apply: apply,
        execution_count: executions.len(),
        executions,
        blockers,
    }
}

pub fn command_to_argv(command: &HostCommandSurface) -> Result<Vec<String>, String> {
    invocation_to_argv(&command.invocation)
}

pub fn invocation_to_argv(invocation: &HostCommandInvocation) -> Result<Vec<String>, String> {
    match invocation {
        HostCommandInvocation::Argv { argv } => Ok(argv.clone()),
        HostCommandInvocation::InstallDir { path, policy } => Ok(vec![
            "install".to_string(),
            "-d".to_string(),
            "-o".to_string(),
            policy.owner.user.to_string(),
            "-g".to_string(),
            policy.owner.group.to_string(),
            "-m".to_string(),
            policy.mode.octal.to_string(),
            path.clone(),
        ]),
        HostCommandInvocation::InstallFile {
            source,
            target,
            policy,
        } => Ok(vec![
            "install".to_string(),
            "-o".to_string(),
            policy.owner.user.to_string(),
            "-g".to_string(),
            policy.owner.group.to_string(),
            "-m".to_string(),
            policy.mode.octal.to_string(),
            source.clone(),
            target.clone(),
        ]),
        HostCommandInvocation::ValidateSudoers { path } => Ok(vec![
            current_runtime_layout().visudo_bin(),
            "-cf".to_string(),
            path.clone(),
        ]),
        HostCommandInvocation::Shell { .. }
        | HostCommandInvocation::Expectation { .. }
        | HostCommandInvocation::PackagePresence { .. } => {
            Err("host execution supports only argv/install/visudo surfaces".to_string())
        }
    }
}

fn report_status(
    executions: &[HostCommandExecution],
    command_count: usize,
    blockers: &[String],
) -> &'static str {
    if !blockers.is_empty() {
        "blocked"
    } else if executions.len() == command_count
        && executions
            .iter()
            .all(|execution| execution.exit_code == Some(0))
    {
        "completed"
    } else {
        "failed"
    }
}

fn step_report_status(
    executions: &[HostExecutionStepExecution],
    command_count: usize,
    blockers: &[String],
) -> &'static str {
    if !blockers.is_empty() {
        "blocked"
    } else if executions.len() == command_count
        && executions.iter().all(|execution| execution.success)
    {
        "completed"
    } else {
        "failed"
    }
}

fn step_accepts_exit_code(step: &HostExecutionStep, exit_code: Option<i32>) -> bool {
    exit_code
        .map(|code| step.success_exit_codes.contains(&code))
        .unwrap_or(false)
}

fn run_argv(role: String, argv: Vec<String>) -> HostCommandExecution {
    match capture_argv(&argv) {
        Ok(output) => HostCommandExecution {
            role,
            argv,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
        },
        Err(error) => HostCommandExecution {
            role,
            argv,
            exit_code: None,
            stdout: String::new(),
            stderr: error,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        command_to_argv, execute_argv_commands, execute_execution_plan, execute_host_commands,
    };
    use crate::command_surface::{expectation, install_dir_command};
    use crate::execution::types::{HostArgvCommand, host_execution_plan_from_commands};
    use crate::install_contract::{MODE_0755, root_owned_path};

    #[test]
    fn install_dir_surface_becomes_argv_without_shell() {
        let command = install_dir_command(
            "prepare_directory",
            "/tmp/cynic-test",
            root_owned_path(MODE_0755),
        );

        let argv = command_to_argv(&command).expect("install_dir argv");

        assert_eq!(
            argv,
            vec![
                "install",
                "-d",
                "-o",
                "root",
                "-g",
                "root",
                "-m",
                "0755",
                "/tmp/cynic-test"
            ]
        );
    }

    #[test]
    fn refuses_non_executable_surfaces() {
        let command = expectation("prove", "human proof");

        let error = command_to_argv(&command).expect_err("expectation must not become argv");

        assert!(error.contains("host execution supports only"));
    }

    #[test]
    fn apply_flag_is_required() {
        let report = execute_host_commands(&[], false, true);

        assert_eq!(report.status, "blocked");
        assert!(report.blockers.contains(&"apply_flag_required".to_string()));
    }

    #[test]
    fn raw_argv_execution_is_blocked_even_when_caller_marks_it_read_only() {
        let report = execute_argv_commands(
            &[HostArgvCommand {
                role: "probe".to_string(),
                argv: vec!["/bin/true".to_string()],
            }],
            true,
            false,
        );

        assert_eq!(report.schema, "cynic.host.execution_report.v1");
        assert_eq!(report.status, "blocked");
        assert_eq!(report.execution_count, 0);
        assert!(
            report
                .blockers
                .contains(&"policy_admission_required".to_string())
        );
    }

    #[test]
    fn execution_plan_carries_layer_privilege_and_step_identity() {
        let plan = host_execution_plan_from_commands(
            "bridge.install",
            "host_exec_bridge",
            vec![install_dir_command(
                "prepare directory",
                "/tmp/cynic-test",
                root_owned_path(MODE_0755),
            )],
            true,
            true,
        );

        assert_eq!(plan.schema, "cynic.host.execution_plan.v1");
        assert_eq!(plan.layer, "host_exec_bridge");
        assert!(plan.mutates);
        assert!(plan.requires_privilege);
        assert_eq!(plan.steps[0].id, "001-prepare_directory");
    }

    #[test]
    fn execution_plan_report_blocks_without_apply() {
        let plan = host_execution_plan_from_commands(
            "probe",
            "host",
            vec![install_dir_command(
                "prepare_directory",
                "/tmp/cynic-test",
                root_owned_path(MODE_0755),
            )],
            true,
            true,
        );

        let report = execute_execution_plan(&plan, false);

        assert_eq!(report.schema, "cynic.host.execution_plan_report.v1");
        assert_eq!(report.status, "blocked");
        assert_eq!(report.plan_id, "probe");
        assert_eq!(report.step_count, 1);
        assert!(report.blockers.contains(&"apply_flag_required".to_string()));
        assert!(
            report
                .blockers
                .contains(&"policy_admission_required".to_string())
        );
    }
}
