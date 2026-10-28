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

