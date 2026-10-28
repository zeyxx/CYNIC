//! Local deploy adapter (migrated from cynic-build).
//!
//! Spawns processes via std::process::Command. Implements cynic-release's
//! external_deploy::DeployPort.

use std::process::Command;

use crate::external_deploy::{
    CommandOutput, DeployKind, DeployPort, DeployRequest, DeployResult, DeployOutput,
};

fn run_local_command_str(command: &str) -> CommandOutput {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return CommandOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: "empty command".to_string(),
            spawn_error: Some("empty command".to_string()),
        };
    }
    let mut cmd = Command::new(parts[0]);
    cmd.args(&parts[1..]);
    match cmd.output() {
        Ok(output) => CommandOutput {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            spawn_error: None,
        },
        Err(e) => CommandOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: format!("spawn error: {e}"),
            spawn_error: Some(e.to_string()),
        },
    }
}

fn run_local_argv(argv: &[String]) -> CommandOutput {
    if argv.is_empty() {
        return CommandOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: "empty command".to_string(),
            spawn_error: Some("empty command".to_string()),
        };
    }
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    match cmd.output() {
        Ok(output) => CommandOutput {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            spawn_error: None,
        },
        Err(e) => CommandOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: format!("spawn error: {e}"),
            spawn_error: Some(e.to_string()),
        },
    }
}

fn build_deploy_result(
    kind: DeployKind,
    request: &DeployRequest,
    artifact_ref: String,
    output: CommandOutput,
) -> DeployResult {
    let success = output.success();
    let error = if success {
        None
    } else {
        output
            .spawn_error
            .clone()
            .or_else(|| {
                let s = output.stderr.trim();
                if s.is_empty() {
                    Some("command failed".to_string())
                } else {
                    Some(s.to_string())
                }
            })
    };
    DeployResult {
        kind,
        target_host_id: request.target_host_id.clone(),
        artifact_ref,
        success,
        output: Some(DeployOutput::from_command_output(&output)),
        error,
    }
}

pub struct LocalDeployAdapter;

impl Default for LocalDeployAdapter {
    fn default() -> Self {
        Self
    }
}

impl DeployPort for LocalDeployAdapter {
    fn deploy_artifact(&self, request: &DeployRequest) -> DeployResult {
        let output = run_local_command_str(&request.artifact_ref);
        build_deploy_result(DeployKind::Deploy, request, request.artifact_ref.clone(), output)
    }

    fn rollback(&self, request: &DeployRequest) -> DeployResult {
        let artifact_ref = request
            .previous_artifact_ref
            .clone()
            .unwrap_or_else(|| request.artifact_ref.clone());
        let output = run_local_command_str(&artifact_ref);
        build_deploy_result(DeployKind::Rollback, request, artifact_ref, output)
    }

    fn health_check(&self, request: &DeployRequest) -> DeployResult {
        let output = match &request.health_check_argv {
            Some(argv) if !argv.is_empty() => run_local_argv(argv),
            _ => run_local_command_str(&request.artifact_ref),
        };
        build_deploy_result(DeployKind::HealthCheck, request, request.artifact_ref.clone(), output)
    }
}

pub type LocalDeploy = LocalDeployAdapter;
pub type LocalDeploymentAdapter = LocalDeployAdapter;
pub type LocalDeployPortAdapter = LocalDeployAdapter;
pub type LocalDeployer = LocalDeployAdapter;