//! HTTP external build adapter.
//!
//! WHY: implements ExternalBuildPort by executing commands on a remote builder
//! via HTTP POST /exec. The trait is pure; this adapter owns all I/O (FCIS).
//! HOW: uses reqwest::blocking::Client on a dedicated OS thread to POST
//! {command, work_dir} to /exec and parse {exit_code, stdout, stderr} into
//! BuildCommandOutput.
//! NEGATIVE BOUNDARY: no domain logic, no admission — those live in
//! crate::external_build.

use crate::builder_client::resolve_builder_url_with_health;
use crate::external_build::{
    ExternalBuildPort, ExternalBuildProof, ExternalBuildRequest, ExternalCommandResult,
    build_external_proof,
};
use crate::ports::BuildCommandOutput;
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
    use crate::external_build::ExternalBuildKind;

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

use cynic_release::{
    DeployKind, DeployPort, DeployRequest, DeployResult, DeployOutput,
};

/// Response body returned by `POST /deploy` on the builder.
#[derive(Debug, Deserialize)]
struct DeployHttpResponse {
    success: Option<bool>,
    artifact_ref: Option<String>,
    error: Option<String>,
    output: Option<DeployOutput>,
}

/// HTTP adapter for `DeployPort`: POSTs to the builder `/deploy` endpoint.
pub struct HttpDeployAdapter {
    /// Base URL of the builder (e.g. "http://10.10.10.119:9090").
    pub builder_url: String,
}

impl HttpDeployAdapter {
    pub fn new(builder_url: String) -> Self {
        Self { builder_url }
    }

    pub fn from_resolved_url() -> Result<Self, String> {
        let url = resolve_builder_url_with_health()?;
        Ok(Self::new(url))
    }

    pub fn deploy_url(&self) -> String {
        format!("{}/deploy", self.builder_url.trim_end_matches('/'))
    }

    fn post_deploy(&self, request: &DeployRequest) -> Result<DeployHttpResponse, String> {
        let url = self.deploy_url();
        let request = request.clone();
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| format!("http client init failed: {e}"))?;
            let res = client
                .post(&url)
                .json(&request)
                .send()
                .map_err(|e| format!("failed to POST /deploy to {url}: {e}"))?;
            if !res.status().is_success() {
                let status = res.status();
                let body = res.text().unwrap_or_default();
                return Err(format!("builder /deploy failed ({status}): {body}"));
            }
            res.json::<DeployHttpResponse>()
                .map_err(|e| format!("failed to parse /deploy response: {e}"))
        })
        .join()
        .map_err(|_| "builder /deploy thread panicked".to_string())?
    }
}

impl DeployPort for HttpDeployAdapter {
    fn deploy_artifact(&self, request: &DeployRequest) -> DeployResult {
        let mut req = request.clone();
        req.kind = DeployKind::Deploy;
        let fallback = req.artifact_ref.clone();
        match self.post_deploy(&req) {
            Ok(resp) => DeployResult {
                kind: DeployKind::Deploy,
                target_host_id: request.target_host_id.clone(),
                artifact_ref: resp.artifact_ref.unwrap_or(fallback),
                success: resp.success.unwrap_or(false),
                output: resp.output,
                error: resp.error,
            },
            Err(e) => DeployResult {
                kind: DeployKind::Deploy,
                target_host_id: request.target_host_id.clone(),
                artifact_ref: fallback,
                success: false,
                output: Some(DeployOutput {
                    stdout: String::new(),
                    stderr: e.clone(),
                    exit_code: None,
                }),
                error: Some(e),
            },
        }
    }

    fn rollback(&self, request: &DeployRequest) -> DeployResult {
        let mut req = request.clone();
        req.kind = DeployKind::Rollback;
        let fallback = req
            .previous_artifact_ref
            .clone()
            .unwrap_or_else(|| req.artifact_ref.clone());
        if req.previous_artifact_ref.is_none() {
            req.previous_artifact_ref = Some(fallback.clone());
        }
        match self.post_deploy(&req) {
            Ok(resp) => DeployResult {
                kind: DeployKind::Rollback,
                target_host_id: request.target_host_id.clone(),
                artifact_ref: resp.artifact_ref.unwrap_or(fallback),
                success: resp.success.unwrap_or(false),
                output: resp.output,
                error: resp.error,
            },
            Err(e) => DeployResult {
                kind: DeployKind::Rollback,
                target_host_id: request.target_host_id.clone(),
                artifact_ref: fallback,
                success: false,
                output: Some(DeployOutput {
                    stdout: String::new(),
                    stderr: e.clone(),
                    exit_code: None,
                }),
                error: Some(e),
            },
        }
    }

    fn health_check(&self, request: &DeployRequest) -> DeployResult {
        let mut req = request.clone();
        req.kind = DeployKind::HealthCheck;
        let fallback = req.artifact_ref.clone();
        match self.post_deploy(&req) {
            Ok(resp) => DeployResult {
                kind: DeployKind::HealthCheck,
                target_host_id: request.target_host_id.clone(),
                artifact_ref: resp.artifact_ref.unwrap_or(fallback),
                success: resp.success.unwrap_or(false),
                output: resp.output,
                error: resp.error,
            },
            Err(e) => DeployResult {
                kind: DeployKind::HealthCheck,
                target_host_id: request.target_host_id.clone(),
                artifact_ref: fallback,
                success: false,
                output: Some(DeployOutput {
                    stdout: String::new(),
                    stderr: e.clone(),
                    exit_code: None,
                }),
                error: Some(e),
            },
        }
    }
}

// Implement DeployPort for the pre-existing external-build HTTP adapter too.
impl DeployPort for HttpExternalBuildAdapter {
    fn deploy_artifact(&self, request: &DeployRequest) -> DeployResult {
        HttpDeployAdapter {
            builder_url: self.builder_url.clone(),
        }
        .deploy_artifact(request)
    }
    fn rollback(&self, request: &DeployRequest) -> DeployResult {
        HttpDeployAdapter {
            builder_url: self.builder_url.clone(),
        }
        .rollback(request)
    }
    fn health_check(&self, request: &DeployRequest) -> DeployResult {
        HttpDeployAdapter {
            builder_url: self.builder_url.clone(),
        }
        .health_check(request)
    }
}

pub type HttpDeploy = HttpDeployAdapter;
pub type HttpDeploymentAdapter = HttpDeployAdapter;
pub type HttpDeployPortAdapter = HttpDeployAdapter;
pub type HttpDeployer = HttpDeployAdapter;

#[cfg(test)]
mod deploy_tests {
    use super::*;
    use crate::deploy::{DeployMutationPolicy, DeployRequest};

    fn sample_request() -> DeployRequest {
        DeployRequest {
            target_host_id: "builder".to_string(),
            artifact_ref: "sha256:abc".to_string(),
            kind: DeployKind::Deploy,
            mutation_policy: DeployMutationPolicy::ExplicitApplyOnly,
            apply: true,
            health_check_argv: Some(vec!["echo".to_string(), "healthy".to_string()]),
            previous_artifact_ref: Some("sha256:def".to_string()),
        }
    }

    #[test]
    fn builder_url_stored() {
        let adapter = HttpDeployAdapter::new("http://10.10.10.119:9090".to_string());
        assert_eq!(adapter.builder_url, "http://10.10.10.119:9090");
    }

    #[test]
    fn deploy_url_appends_deploy_suffix() {
        let adapter = HttpDeployAdapter::new("http://10.10.10.119:9090".to_string());
        assert_eq!(adapter.deploy_url(), "http://10.10.10.119:9090/deploy");
    }

    #[test]
    fn deploy_url_strips_trailing_slash() {
        let adapter = HttpDeployAdapter::new("http://10.10.10.119:9090/".to_string());
        assert_eq!(adapter.deploy_url(), "http://10.10.10.119:9090/deploy");
    }

    #[test]
    fn deploy_artifact_maps_connection_error() {
        let adapter = HttpDeployAdapter::new("http://127.0.0.1:1".to_string());
        let result = adapter.deploy_artifact(&sample_request());
        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("failed to POST /deploy"));
    }

    #[test]
    fn rollback_maps_connection_error() {
        let adapter = HttpDeployAdapter::new("http://127.0.0.1:1".to_string());
        let result = adapter.rollback(&sample_request());
        assert!(!result.success);
        assert!(result.error.unwrap().contains("failed to POST /deploy"));
    }

    #[test]
    fn health_check_maps_connection_error() {
        let adapter = HttpDeployAdapter::new("http://127.0.0.1:1".to_string());
        let result = adapter.health_check(&sample_request());
        assert!(!result.success);
        assert!(result.error.unwrap().contains("failed to POST /deploy"));
    }

    #[test]
    fn external_build_adapter_also_deploys() {
        let adapter = HttpExternalBuildAdapter::new("http://127.0.0.1:1".to_string());
        let result = adapter.deploy_artifact(&sample_request());
        assert!(!result.success);
    }
}
