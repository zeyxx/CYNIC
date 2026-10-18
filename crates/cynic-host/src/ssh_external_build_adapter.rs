//! SSH external build adapter for cynic-host.
//!
//! WHY: implements ExternalBuildPort by executing commands on a remote builder
//! via SSH subprocess. This is the bottom-up pattern: observe first, then
//! scale to N builders. The trait is pure; this adapter owns all I/O.
//! HOW: uses std::process::Command with ssh. Pattern from source_sync/apply.rs.
//! NEGATIVE BOUNDARY: no domain logic, no admission — those live in
//! cynic-build::external_build.

use std::process::Command;

use cynic_build::external_build::{
    ExternalBuildPort, ExternalBuildProof, ExternalBuildRequest, ExternalCommandResult,
    build_external_proof,
};
use cynic_build::ports::BuildCommandOutput;

/// SSH adapter: executes build/test commands on a remote builder.
pub struct SshExternalBuildAdapter {
    /// SSH target (e.g. "cynic@10.10.10.119").
    pub ssh_target: String,
    /// Optional SSH key path.
    pub ssh_key: Option<String>,
}

impl SshExternalBuildAdapter {
    pub fn new(ssh_target: String, ssh_key: Option<String>) -> Self {
        Self {
            ssh_target,
            ssh_key,
        }
    }

    fn ssh_command(&self, remote_cmd: &str) -> Command {
        let mut cmd = Command::new("ssh");
        if let Some(key) = &self.ssh_key {
            cmd.args(["-i", key]);
        }
        cmd.args([
            "-o",
            "IdentitiesOnly=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "BatchMode=yes",
            &self.ssh_target,
            remote_cmd,
        ]);
        cmd
    }
}

impl ExternalBuildPort for SshExternalBuildAdapter {
    fn run_command(&self, command: &str, work_dir: &str) -> BuildCommandOutput {
        let remote_cmd = if work_dir.is_empty() {
            command.to_string()
        } else {
            format!("cd {} && {}", work_dir, command)
        };

        let mut cmd = self.ssh_command(&remote_cmd);
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
                stderr: format!("ssh spawn error: {e}"),
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

    #[test]
    fn ssh_target_stored() {
        let adapter = SshExternalBuildAdapter::new(
            "cynic@10.10.10.119".to_string(),
            Some("/home/user/.ssh/id_ed25519".to_string()),
        );
        assert_eq!(adapter.ssh_target, "cynic@10.10.10.119");
        assert!(adapter.ssh_key.is_some());
    }

    #[test]
    fn ssh_command_includes_target() {
        let adapter = SshExternalBuildAdapter::new("cynic@10.10.10.119".to_string(), None);
        let cmd = adapter.ssh_command("echo hello");
        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();
        assert!(args.contains(&"cynic@10.10.10.119".to_string()));
        assert!(args.contains(&"echo hello".to_string()));
    }
}