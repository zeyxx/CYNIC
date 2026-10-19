//! Local external build adapter.
//!
//! WHY: implements ExternalBuildPort by executing commands locally.
//! All I/O lives here (adapter layer, FCIS).
//! HOW: uses std::process::Command for command execution. No domain logic.
//! NEGATIVE BOUNDARY: no pure predicates, no admission logic — those live in
//! crate::external_build.

use std::process::Command;

use crate::external_build::{
    ExternalBuildPort, ExternalBuildProof, ExternalBuildRequest, ExternalCommandResult,
    build_external_proof,
};
use crate::ports::BuildCommandOutput;

/// Local adapter: executes build/test commands on the current host.
/// For builder execution, use HttpExternalBuildAdapter (HTTP /exec).
pub struct LocalExternalBuildAdapter;

impl ExternalBuildPort for LocalExternalBuildAdapter {
    fn run_command(&self, command: &str, work_dir: &str) -> BuildCommandOutput {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return BuildCommandOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: "empty command".to_string(),
                spawn_error: Some("empty command".to_string()),
            };
        }

        let mut cmd = Command::new(parts[0]);
        cmd.args(&parts[1..]);
        if !work_dir.is_empty() {
            cmd.current_dir(work_dir);
        }

        match cmd.output() {
            Ok(output) => BuildCommandOutput {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                spawn_error: None,
            },
            Err(e) => BuildCommandOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("spawn error: {e}"),
                spawn_error: Some(e.to_string()),
            },
        }
    }

    fn run_build_proof(
        &self,
        request: &ExternalBuildRequest,
        builder_node_id: &str,
    ) -> ExternalBuildProof {
        let mut results = Vec::new();
        for cmd in &request.commands {
            let output = self.run_command(cmd, &request.work_dir);
            results.push(ExternalCommandResult::from_output(cmd.clone(), &output));
        }
        build_external_proof(&request.project_id, request.kind, results, builder_node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_build::ExternalBuildKind;

    #[test]
    fn run_command_echo() {
        let adapter = LocalExternalBuildAdapter;
        let output = adapter.run_command("echo hello", "");
        assert!(output.success());
        assert!(output.stdout.contains("hello"));
    }

    #[test]
    fn run_command_fail() {
        let adapter = LocalExternalBuildAdapter;
        let output = adapter.run_command("false", "");
        assert!(!output.success());
    }

    #[test]
    fn run_build_proof_all_pass() {
        let adapter = LocalExternalBuildAdapter;
        let request = ExternalBuildRequest {
            project_id: "test".to_string(),
            commands: vec!["echo ok".to_string()],
            work_dir: String::new(),
            kind: ExternalBuildKind::Build,
            builder_node_id: "local".to_string(),
        };
        let proof = adapter.run_build_proof(&request, "local");
        assert!(proof.overall_success);
        assert_eq!(proof.commands_run, 1);
    }
}

use crate::deploy::{
    DeployKind, DeployPort, DeployRequest, DeployResult, DeployOutput,
};

/// Run a command string by splitting on whitespace (mirrors the external build
/// adapter). Returns a `BuildCommandOutput` so callers can map it uniformly.
fn run_local_command_str(command: &str) -> crate::ports::BuildCommandOutput {
    use std::process::Command;
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return crate::ports::BuildCommandOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: "empty command".to_string(),
            spawn_error: Some("empty command".to_string()),
        };
    }
    let mut cmd = Command::new(parts[0]);
    cmd.args(&parts[1..]);
    match cmd.output() {
        Ok(output) => crate::ports::BuildCommandOutput {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            spawn_error: None,
        },
        Err(e) => crate::ports::BuildCommandOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: format!("spawn error: {e}"),
            spawn_error: Some(e.to_string()),
        },
    }
}

fn run_local_argv(argv: &[String]) -> crate::ports::BuildCommandOutput {
    use std::process::Command;
    if argv.is_empty() {
        return crate::ports::BuildCommandOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: "empty command".to_string(),
            spawn_error: Some("empty command".to_string()),
        };
    }
    let mut cmd = Command::new(&argv[0]);
    cmd.args(&argv[1..]);
    match cmd.output() {
        Ok(output) => crate::ports::BuildCommandOutput {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            spawn_error: None,
        },
        Err(e) => crate::ports::BuildCommandOutput {
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
    output: crate::ports::BuildCommandOutput,
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

/// Local adapter for `DeployPort`: spawns processes via `std::process::Command`.
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

// Implement DeployPort for the pre-existing external-build local adapter as well,
// so callers that already hold a `LocalExternalBuildAdapter` can deploy too.
impl DeployPort for LocalExternalBuildAdapter {
    fn deploy_artifact(&self, request: &DeployRequest) -> DeployResult {
        LocalDeployAdapter.deploy_artifact(request)
    }
    fn rollback(&self, request: &DeployRequest) -> DeployResult {
        LocalDeployAdapter.rollback(request)
    }
    fn health_check(&self, request: &DeployRequest) -> DeployResult {
        LocalDeployAdapter.health_check(request)
    }
}

pub type LocalDeploy = LocalDeployAdapter;
pub type LocalDeploymentAdapter = LocalDeployAdapter;
pub type LocalDeployPortAdapter = LocalDeployAdapter;
pub type LocalDeployer = LocalDeployAdapter;

#[cfg(test)]
mod deploy_tests {
    use super::*;
    use crate::deploy::{DeployMutationPolicy, DeployRequest};

    fn sample_request() -> DeployRequest {
        DeployRequest {
            target_host_id: "local-host".to_string(),
            artifact_ref: "echo deployed".to_string(),
            kind: DeployKind::Deploy,
            mutation_policy: DeployMutationPolicy::ExplicitApplyOnly,
            apply: true,
            health_check_argv: Some(vec!["echo".to_string(), "healthy".to_string()]),
            previous_artifact_ref: Some("echo rolled-back".to_string()),
        }
    }

    #[test]
    fn deploy_artifact_success() {
        let adapter = LocalDeployAdapter;
        let result = adapter.deploy_artifact(&sample_request());
        assert!(result.success);
        assert!(result.output.unwrap().stdout.contains("deployed"));
    }

    #[test]
    fn deploy_artifact_failure() {
        let adapter = LocalDeployAdapter;
        let mut req = sample_request();
        req.artifact_ref = "false".to_string();
        let result = adapter.deploy_artifact(&req);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn rollback_uses_previous_ref() {
        let adapter = LocalDeployAdapter;
        let result = adapter.rollback(&sample_request());
        assert!(result.success);
        assert!(result.output.unwrap().stdout.contains("rolled-back"));
        assert_eq!(result.artifact_ref, "echo rolled-back");
    }

    #[test]
    fn health_check_runs_argv() {
        let adapter = LocalDeployAdapter;
        let result = adapter.health_check(&sample_request());
        assert!(result.success);
        assert!(result.output.unwrap().stdout.contains("healthy"));
    }

    #[test]
    fn health_check_failure() {
        let adapter = LocalDeployAdapter;
        let mut req = sample_request();
        req.health_check_argv = Some(vec!["false".to_string()]);
        let result = adapter.health_check(&req);
        assert!(!result.success);
    }

    #[test]
    fn external_build_adapter_also_deploys() {
        let adapter = LocalExternalBuildAdapter;
        let result = adapter.deploy_artifact(&sample_request());
        assert!(result.success);
    }
}
