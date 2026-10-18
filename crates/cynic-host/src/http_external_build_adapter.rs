//! HTTP external build adapter for cynic-host.
//!
//! WHY: implements ExternalBuildPort by executing commands on a remote builder
//! via HTTP POST /exec. Replaces the SSH-subprocess transport with the builder's
//! HTTP API, eliminating the SSH dependency from cynic-host. The trait is pure;
//! this adapter owns all I/O (FCIS).
//! HOW: uses reqwest::blocking::Client on a dedicated OS thread (the proven
//! pattern from cynic_build::submit_workload) to POST {command, work_dir} to
//! /exec and parse {exit_code, stdout, stderr} into BuildCommandOutput.
//! NEGATIVE BOUNDARY: no domain logic, no admission — those live in
//! cynic-build::external_build.

use cynic_build::builder_client::resolve_builder_url_with_health;
use cynic_build::external_build::{
    ExternalBuildPort, ExternalBuildProof, ExternalBuildRequest, ExternalCommandResult,
    build_external_proof,
};
use cynic_build::ports::BuildCommandOutput;
use serde::Deserialize;

/// Response body returned by `POST /exec` on the builder.
#[derive(Debug, Deserialize)]
struct ExecResponse {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// HTTP adapter: executes build/test commands on a remote builder via HTTP.
///
/// The builder URL is either supplied explicitly or resolved dynamically via
/// [`resolve_builder_url_with_health`] (CYNIC_BUILDER_URL env, then pool + health-check).
pub struct HttpExternalBuildAdapter {
    /// Base URL of the builder (e.g. "http://10.10.10.119:9090").
    pub builder_url: String,
}

impl HttpExternalBuildAdapter {
    pub fn new(builder_url: String) -> Self {
        Self { builder_url }
    }

    /// Resolve the builder URL dynamically (pool + health-check) and construct the adapter.
    pub fn from_resolved_url() -> Result<Self, String> {
        let url = resolve_builder_url_with_health()?;
        Ok(Self::new(url))
    }

    fn exec_url(&self) -> String {
        format!("{}/exec", self.builder_url.trim_end_matches('/'))
    }

    /// POST {command, work_dir} to /exec and parse the response.
    ///
    /// Runs on a dedicated OS thread because reqwest::blocking::Client cannot
    /// be constructed inside an async runtime context.
    fn post_exec(&self, command: &str, work_dir: &str) -> Result<ExecResponse, String> {
        let url = self.exec_url();
        let command = command.to_string();
        let work_dir = work_dir.to_string();
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| format!("http client init failed: {e}"))?;
            let payload = serde_json::json!({
                "command": command,
                "work_dir": work_dir,
            });
            let res = client
                .post(&url)
                .json(&payload)
                .send()
                .map_err(|e| format!("failed to POST /exec to {url}: {e}"))?;
            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().unwrap_or_default();
                return Err(format!("builder /exec failed ({status}): {body}"));
            }
            res.json::<ExecResponse>()
                .map_err(|e| format!("failed to parse /exec response: {e}"))
        })
        .join()
        .map_err(|_| "builder /exec thread panicked".to_string())?
    }
}

impl ExternalBuildPort for HttpExternalBuildAdapter {
    fn run_command(&self, command: &str, work_dir: &str) -> BuildCommandOutput {
        match self.post_exec(command, work_dir) {
            Ok(resp) => BuildCommandOutput {
                exit_code: resp.exit_code,
                stdout: resp.stdout,
                stderr: resp.stderr,
                spawn_error: None,
            },
            Err(e) => BuildCommandOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: e.clone(),
                spawn_error: Some(e),
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
    use cynic_build::external_build::ExternalBuildKind;

    #[test]
    fn builder_url_stored() {
        let adapter = HttpExternalBuildAdapter::new("http://10.10.10.119:9090".to_string());
        assert_eq!(adapter.builder_url, "http://10.10.10.119:9090");
    }

    #[test]
    fn exec_url_appends_exec_suffix() {
        let adapter = HttpExternalBuildAdapter::new("http://10.10.10.119:9090".to_string());
        assert_eq!(adapter.exec_url(), "http://10.10.10.119:9090/exec");
    }

    #[test]
    fn exec_url_strips_trailing_slash() {
        let adapter = HttpExternalBuildAdapter::new("http://10.10.10.119:9090/".to_string());
        assert_eq!(adapter.exec_url(), "http://10.10.10.119:9090/exec");
    }

    #[test]
    fn run_command_maps_connection_error_to_spawn_error() {
        // Unreachable builder: must produce a spawn_error BuildCommandOutput,
        // never a panic — the failure is data, not a crash.
        let adapter = HttpExternalBuildAdapter::new("http://127.0.0.1:1".to_string());
        let output = adapter.run_command("echo hello", "");
        assert!(!output.success());
        assert!(output.spawn_error.is_some());
        assert!(output.stderr.contains("failed to POST /exec"));
    }

    #[test]
    fn run_build_proof_collects_results() {
        // Unreachable builder: every command fails, proof is still built.
        let adapter = HttpExternalBuildAdapter::new("http://127.0.0.1:1".to_string());
        let request = ExternalBuildRequest {
            project_id: "test".to_string(),
            commands: vec!["echo one".to_string(), "echo two".to_string()],
            work_dir: String::new(),
            kind: ExternalBuildKind::Build,
            builder_node_id: "test-builder".to_string(),
        };
        let proof = adapter.run_build_proof(&request, "test-builder");
        assert_eq!(proof.commands_run, 2);
        assert_eq!(proof.commands_passed, 0);
        assert_eq!(proof.commands_failed, 2);
        assert!(!proof.overall_success);
    }
}
