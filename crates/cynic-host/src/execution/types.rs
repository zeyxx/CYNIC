use serde::Serialize;

use crate::command_surface::HostCommandSurface;

#[derive(Debug, Clone, Serialize)]
pub struct HostExecutionReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub requested_apply: bool,
    pub execution_count: usize,
    pub executions: Vec<HostCommandExecution>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostCommandExecution {
    pub role: String,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostExecutionPlan {
    pub schema: &'static str,
    pub id: String,
    pub layer: String,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub steps: Vec<HostExecutionStep>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostExecutionStep {
    pub id: String,
    pub role: String,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub command: HostCommandSurface,
    pub success_exit_codes: Vec<i32>,
    pub preconditions: Vec<String>,
    pub proof_paths: Vec<String>,
    pub rollback_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostExecutionPlanReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub plan_id: String,
    pub layer: String,
    pub mutates: bool,
    pub requires_privilege: bool,
    pub requested_apply: bool,
    pub step_count: usize,
    pub execution_count: usize,
    pub executions: Vec<HostExecutionStepExecution>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostExecutionStepExecution {
    pub step_id: String,
    pub role: String,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub proof_paths: Vec<String>,
    pub rollback_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HostArgvCommand {
    pub role: String,
    pub argv: Vec<String>,
}

pub fn host_execution_plan_from_commands(
    id: impl Into<String>,
    layer: impl Into<String>,
    commands: Vec<HostCommandSurface>,
    mutates: bool,
    requires_privilege: bool,
) -> HostExecutionPlan {
    let steps = commands
        .into_iter()
        .enumerate()
        .map(|(index, command)| HostExecutionStep {
            id: step_id(index, &command.role),
            role: command.role.clone(),
            mutates,
            requires_privilege,
            command,
            success_exit_codes: vec![0],
            preconditions: Vec::new(),
            proof_paths: Vec::new(),
            rollback_hint: None,
        })
        .collect();

    HostExecutionPlan {
        schema: "cynic.host.execution_plan.v1",
        id: id.into(),
        layer: layer.into(),
        mutates,
        requires_privilege,
        steps,
    }
}

fn step_id(index: usize, role: &str) -> String {
    let normalized = role
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{:03}-{normalized}", index + 1)
}
