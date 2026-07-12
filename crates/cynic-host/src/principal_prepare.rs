use serde::Serialize;

use crate::command_surface::{HostCommandInvocation, HostCommandSurface};
use crate::domain::principal::PrincipalPolicy;
use crate::execution::{
    HostCommandExecution, HostExecutionPlan, HostExecutionStep, execute_execution_plan,
    host_execution_plan_from_commands,
};
use crate::runtime_layout::current_runtime_layout;

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalPrepareReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub policy: PrincipalPreparePolicyView,
    pub execution_plan: HostExecutionPlan,
    pub commands: Vec<PrincipalPrepareCommand>,
    pub executions: Vec<PrincipalPrepareExecution>,
    pub blockers: Vec<&'static str>,
    pub safety_rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalPreparePolicyView {
    pub runtime_user: String,
    pub runtime_group: String,
    pub home_dir: String,
    pub config_dir: String,
    pub state_dirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalPrepareCommand {
    pub role: &'static str,
    pub kind: &'static str,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrincipalPrepareExecution {
    pub role: String,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub fn principal_prepare_plan(policy: &PrincipalPolicy) -> PrincipalPrepareReport {
    principal_prepare_report(policy, false)
}

pub fn principal_prepare_apply(policy: &PrincipalPolicy) -> PrincipalPrepareReport {
    principal_prepare_report(policy, true)
}

fn principal_prepare_report(policy: &PrincipalPolicy, apply: bool) -> PrincipalPrepareReport {
    let commands = principal_prepare_commands(policy);
    let execution_plan = principal_prepare_execution_plan(&commands);
    let executions = if apply {
        run_commands_with_engine(&execution_plan)
    } else {
        Vec::new()
    };
    let failed = executions
        .iter()
        .any(|execution| execution.exit_code != Some(0));

    PrincipalPrepareReport {
        schema: "cynic.host.principal_prepare.v1",
        status: if !apply {
            "planned"
        } else if failed {
            "failed"
        } else {
            "completed"
        },
        mutates: apply,
        requested_apply: apply,
        policy: PrincipalPreparePolicyView {
            runtime_user: policy.runtime_user.clone(),
            runtime_group: policy.runtime_group.clone(),
            home_dir: policy.home_dir.clone(),
            config_dir: policy.config_dir.clone(),
            state_dirs: policy.state_dirs.clone(),
        },
        execution_plan,
        commands,
        executions,
        blockers: Vec::new(),
        safety_rules: vec![
            "principal prepare must be invoked explicitly; lab image fetch never creates users",
            "commands are argv-only and stop at first failure",
            "runtime user and group names come from cynic-host PrincipalPolicy",
            "state/config directories are owned by the runtime principal after prepare",
        ],
    }
}

fn principal_prepare_commands(policy: &PrincipalPolicy) -> Vec<PrincipalPrepareCommand> {
    let layout = current_runtime_layout();
    let mut commands = vec![
        PrincipalPrepareCommand {
            role: "ensure_runtime_group",
            kind: "argv",
            argv: vec![
                "groupadd".to_string(),
                "--system".to_string(),
                "--force".to_string(),
                policy.runtime_group.clone(),
            ],
        },
        PrincipalPrepareCommand {
            role: "ensure_runtime_user",
            kind: "argv",
            argv: vec![
                "useradd".to_string(),
                "--system".to_string(),
                "--home-dir".to_string(),
                policy.home_dir.clone(),
                "--create-home".to_string(),
                "--gid".to_string(),
                policy.runtime_group.clone(),
                "--shell".to_string(),
                "/usr/sbin/nologin".to_string(),
                policy.runtime_user.clone(),
            ],
        },
        install_dir_command(
            "ensure_home_dir",
            &policy.home_dir,
            &policy.runtime_user,
            &policy.runtime_group,
        ),
        install_dir_command(
            "ensure_config_dir",
            &policy.config_dir,
            &policy.runtime_user,
            &policy.runtime_group,
        ),
    ];
    for path in &policy.state_dirs {
        commands.push(install_dir_command(
            "ensure_state_dir",
            path,
            &policy.runtime_user,
            &policy.runtime_group,
        ));
    }
    commands.push(PrincipalPrepareCommand {
        role: "query_runtime_user",
        kind: "argv",
        argv: vec![
            layout.getent_bin(),
            "passwd".to_string(),
            policy.runtime_user.clone(),
        ],
    });
    commands
}

fn install_dir_command(
    role: &'static str,
    path: &str,
    user: &str,
    group: &str,
) -> PrincipalPrepareCommand {
    PrincipalPrepareCommand {
        role,
        kind: "argv",
        argv: vec![
            "install".to_string(),
            "-d".to_string(),
            "-o".to_string(),
            user.to_string(),
            "-g".to_string(),
            group.to_string(),
            "-m".to_string(),
            "0755".to_string(),
            path.to_string(),
        ],
    }
}

fn principal_prepare_execution_plan(commands: &[PrincipalPrepareCommand]) -> HostExecutionPlan {
    let surfaces = commands
        .iter()
        .map(principal_command_surface)
        .collect::<Vec<_>>();

    let mut plan = host_execution_plan_from_commands(
        "cynic.host.principal_prepare",
        "host_principal",
        surfaces,
        true,
        true,
    );

    for step in &mut plan.steps {
        annotate_principal_prepare_step(step);
    }

    plan
}

fn annotate_principal_prepare_step(step: &mut HostExecutionStep) {
    match &step.command.invocation {
        HostCommandInvocation::Argv { argv } if step.role == "ensure_runtime_group" => {
            if let Some(group) = argv.last() {
                step.proof_paths.push(format!("group:{group}"));
            }
            step.rollback_hint = Some(
                "remove runtime group only after all owned files and users are removed".to_string(),
            );
        }
        HostCommandInvocation::Argv { argv } if step.role == "ensure_runtime_user" => {
            step.success_exit_codes = vec![0, 9];
            if let Some(user) = argv.last() {
                step.proof_paths.push(format!("passwd:{user}"));
            }
            step.rollback_hint =
                Some("remove runtime user only after disabling cynic services".to_string());
        }
        HostCommandInvocation::Argv { argv } if step.role == "query_runtime_user" => {
            if let Some(user) = argv.last() {
                step.proof_paths.push(format!("passwd:{user}"));
            }
        }
        HostCommandInvocation::Argv { argv } if step.role.starts_with("ensure_") => {
            if let Some(path) = argv.last() {
                step.proof_paths.push(path.clone());
                step.rollback_hint =
                    Some("remove directory only if empty and not shared".to_string());
            }
        }
        _ => {}
    }
}

fn run_commands_with_engine(plan: &HostExecutionPlan) -> Vec<PrincipalPrepareExecution> {
    execute_execution_plan(plan, true)
        .executions
        .into_iter()
        .map(|execution| {
            normalize_execution(HostCommandExecution {
                role: execution.role,
                argv: execution.argv,
                exit_code: execution.exit_code,
                stdout: execution.stdout,
                stderr: execution.stderr,
            })
        })
        .collect()
}

fn principal_command_surface(command: &PrincipalPrepareCommand) -> HostCommandSurface {
    HostCommandSurface {
        role: command.role.to_string(),
        invocation: HostCommandInvocation::Argv {
            argv: command.argv.clone(),
        },
    }
}

fn normalize_execution(execution: HostCommandExecution) -> PrincipalPrepareExecution {
    let normalized_exit_code = if execution.role == "ensure_runtime_user"
        && execution.exit_code == Some(9)
        && execution.stderr.contains("already exists")
    {
        Some(0)
    } else {
        execution.exit_code
    };

    PrincipalPrepareExecution {
        role: execution.role,
        argv: execution.argv,
        exit_code: normalized_exit_code,
        stdout: execution.stdout,
        stderr: execution.stderr,
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_execution, principal_prepare_plan};
    use crate::domain::principal::PrincipalPolicy;
    use crate::execution::HostCommandExecution;

    #[test]
    fn principal_prepare_plan_contains_user_and_owned_dirs() {
        let report = principal_prepare_plan(&PrincipalPolicy {
            runtime_user: "cynic".to_string(),
            runtime_group: "cynic".to_string(),
            home_dir: "/home/cynic".to_string(),
            config_dir: "/etc/cynic".to_string(),
            state_dirs: vec!["/var/lib/cynic".to_string(), "/var/log/cynic".to_string()],
        });

        assert_eq!(report.schema, "cynic.host.principal_prepare.v1");
        assert_eq!(report.status, "planned");
        assert!(!report.mutates);
        assert!(
            report
                .commands
                .iter()
                .any(|command| command.role == "ensure_runtime_user")
        );
        assert!(report.commands.iter().any(|command| {
            command.role == "ensure_state_dir"
                && command.argv.iter().any(|part| part == "/var/lib/cynic")
        }));
        assert_eq!(report.execution_plan.schema, "cynic.host.execution_plan.v1");
        assert_eq!(report.execution_plan.id, "cynic.host.principal_prepare");
        assert_eq!(report.execution_plan.layer, "host_principal");
        assert!(report.execution_plan.requires_privilege);
        assert!(report.execution_plan.steps.iter().any(|step| {
            step.role == "ensure_runtime_user"
                && step.success_exit_codes == vec![0, 9]
                && step.proof_paths.contains(&"passwd:cynic".to_string())
        }));
        assert!(report.execution_plan.steps.iter().any(|step| {
            step.role == "ensure_state_dir"
                && step.proof_paths.contains(&"/var/lib/cynic".to_string())
        }));
    }

    #[test]
    fn existing_runtime_user_is_idempotent_success() {
        let execution = normalize_execution(HostCommandExecution {
            role: "ensure_runtime_user".to_string(),
            argv: vec!["useradd".to_string(), "cynic".to_string()],
            exit_code: Some(9),
            stdout: String::new(),
            stderr: "useradd: user 'cynic' already exists".to_string(),
        });

        assert_eq!(execution.exit_code, Some(0));
    }
}
