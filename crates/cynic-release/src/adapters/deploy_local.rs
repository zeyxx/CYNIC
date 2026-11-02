//! Local deploy adapter (migrated from cynic-build).
//!
//! Spawns processes via std::process::Command. Implements cynic-release's
//! external_deploy::DeployPort.

use cynic_process::{ArgvRunner, LocalArgvRunner};

use crate::external_deploy::{
    ArgvOutput, DeployKind, DeployPort, DeployRequest, DeployResult, DeployOutput,
};

fn run_local_command_str(command: &str) -> ArgvOutput {
    let parts: Vec<&str> = command.split_whitespace().collect();
    LocalArgvRunner.run(&parts)
}

fn run_local_argv(argv: &[String]) -> ArgvOutput {
    let parts: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
    LocalArgvRunner.run(&parts)
}

fn build_deploy_result(
    kind: DeployKind,
    request: &DeployRequest,
    artifact_ref: String,
    output: ArgvOutput,
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
        output: Some(DeployOutput::from_argv_output(&output)),
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