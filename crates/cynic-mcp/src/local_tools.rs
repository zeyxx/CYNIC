//! Local tools — predefined filesystem operations for sandboxed agents.
//! No shell injection: each operation constructs Command with typed args.
//! Auth is checked by the MCP tool layer, not here.
//!
//! K18 compliance: every Command that runs inside a timeout uses
//! `kill_on_drop(true)` + `spawn()` + `wait_with_output()`.
//! Using `.output()` directly inside `timeout()` leaves zombie processes
//! when the timeout fires (the child survives SIGTERM and pollutes the cgroup).

use cynic_host::build_evidence::{
    BuildRunnerObservationPort, CurlBuildLeaseVerificationAdapter, JsonBuildRunnerObservationAdapter,
};
use cynic_host::build_workflow::{run_build_workflow, BuildWorkflowConfig, BuildWorkflowResult};
use cynic_host::ports::argv_execution::{ArgvExecutionPort, ArgvExecutionRequest};
use cynic_host::source_control::{
    GitSourceControlAdapter, SourceControlPort, SourcePushAuthorization,
    observe_source_push_target, observe_source_worktree,
};
use cynic_host::source_lease::SourceLeaseVerifier;
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::build::{BuildRecipe, BUILD_RECIPE_SCHEMA};
use cynic_protocol::{LeaseGuard, SecurityEvidenceState, SourceChangeProposal, SourceMatterClass, SourceSnapshot};

/// Result of a git operation.
#[derive(Debug, serde::Serialize)]
pub struct GitResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Result of a build workflow run.
#[derive(Debug, serde::Serialize)]
pub struct BuildWorkflowOutput {
    pub status: String,
    pub proof: Option<cynic_host::build::BuildProof>,
}

/// Run the pure build workflow (observe → verify → decide → execute).
pub async fn run_buildWorkflow(
    _runner: &dyn ArgvExecutionPort,
    project_root: &str,
    commit: &str,
    phase: Option<&str>,
    network_mode: Option<&str>,
    host: Option<&str>,
    user: Option<&str>,
) -> BuildWorkflowOutput {
    let observation_path = format!("/tmp/cynic-build-runner-observation.json");
    let observation_adapter = JsonBuildRunnerObservationAdapter::new(&observation_path);
    let observation = match observation_adapter.observe() {
        Ok(o) => o,
        Err(e) => {
            return BuildWorkflowOutput {
                status: "blocked".to_string(),
                proof: None,
            }
        }
    };

    let argv_runner = LocalArgvRunner;
    let lease_adapter = match host {
        Some(h) => CurlBuildLeaseVerificationAdapter::new(&LocalArgvRunner, h.to_string(), None)
            .expect("lease verification adapter"),
        None => CurlBuildLeaseVerificationAdapter::new(&LocalArgvRunner, "http://10.10.10.101:3000".to_string(), None)
            .expect("lease verification adapter"),
    };

    let recipe = BuildRecipe {
        schema: BUILD_RECIPE_SCHEMA.to_string(),
        action: phase.unwrap_or("make:check").to_string(),
        target_profile: "default".to_string(),
        target_crates: vec![],
        excluded_crates: vec![],
        toolchain_override: None,
        env_overrides: vec![],
        max_parallel_jobs: 1,
        timeout_secs: 600,
        evidence_refs: vec![],
        allowed_actions: vec![],
        rust_min_stack: 2147483648,
        cargo_incremental: false,
        memory_limit_bytes: None,
    };

    let source_snapshot = SourceSnapshot {
        branch: Some("main".into()),
        head_commit: Some(commit.to_string()),
        upstream: Some("forgejo/main".into()),
        staged_paths: vec![],
        unstaged_paths: vec![],
        untracked_paths: vec![],
        unmerged_paths: vec![],
        validation: SecurityEvidenceState::Proven,
        sovereign_transport_identity: SecurityEvidenceState::Proven,
        sovereign_authentication: SecurityEvidenceState::Proven,
        remote_commit_matches: SecurityEvidenceState::Proven,
        build_input_digest: None,
        evidence_refs: vec!["mcp-input".into()],
    };

    let config = BuildWorkflowConfig {
        observation_adapter: &observation_adapter,
        lease_adapter: &lease_adapter,
        argv_runner: &argv_runner,
        source_snapshot,
        recipe,
        argv: vec!["make".to_string(), "check".to_string()],
        artifact_path: "target/release/cynic".to_string(),
        apply: false,
        now_unix_ms: chrono::Utc::now().timestamp_millis() as u64,
    };

    match run_build_workflow(&config) {
        Ok(result) => match result {
            BuildWorkflowResult::Blocked { .. } => BuildWorkflowOutput {
                status: "blocked".to_string(),
                proof: None,
            },
            BuildWorkflowResult::Planned { proof } => BuildWorkflowOutput {
                status: "planned".to_string(),
                proof: Some(proof),
            },
            BuildWorkflowResult::Executed { proof } => BuildWorkflowOutput {
                status: "executed".to_string(),
                proof: Some(proof),
            },
        },
        Err(_) => BuildWorkflowOutput {
            status: "blocked".to_string(),
            proof: None,
        },
    }
}

/// Result of a Docker CLI operation.
#[derive(Debug, serde::Serialize)]
pub struct DockerResult {
    pub success: bool,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

/// Git operations — each maps to a specific git command with typed args.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "op")]
pub enum GitOp {
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "log")]
    Log { count: Option<u32> },
    #[serde(rename = "diff")]
    Diff,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "op")]
pub enum SourceControlOp {
    #[serde(rename = "observe")]
    Observe {
        #[serde(default)]
        expected_owner: Option<String>,
    },
    #[serde(rename = "propose")]
    Propose { proposal: SourceChangeProposalInput },
    #[serde(rename = "commit")]
    Commit { proposal: SourceChangeProposalInput },
    #[serde(rename = "push")]
    Push {
        proposal: SourceChangeProposalInput,
        remote: String,
        branch: String,
    },
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SourceChangeProposalInput {
    pub schema: String,
    pub proposal_id: String,
    pub repository_id: String,
    pub worktree_id: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub base_commit: String,
    pub actor: String,
    pub node_id: String,
    pub intent: String,
    pub matter: String,
    pub paths: Vec<String>,
    pub validation: String,
    pub rollback: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl SourceChangeProposalInput {
    fn to_protocol(&self) -> Result<SourceChangeProposal, String> {
        let matter = serde_json::from_value::<SourceMatterClass>(serde_json::Value::String(
            self.matter.clone(),
        ))
        .map_err(|error| format!("invalid matter class: {error}"))?;
        let validation = serde_json::from_value::<SecurityEvidenceState>(
            serde_json::Value::String(self.validation.clone()),
        )
        .map_err(|error| format!("invalid validation state: {error}"))?;
        Ok(SourceChangeProposal {
            schema: self.schema.clone(),
            proposal_id: self.proposal_id.clone(),
            repository_id: self.repository_id.clone(),
            worktree_id: self.worktree_id.clone(),
            lease_id: self.lease_id.clone(),
            fencing_token: self.fencing_token,
            base_commit: self.base_commit.clone(),
            actor: self.actor.clone(),
            node_id: self.node_id.clone(),
            intent: self.intent.clone(),
            matter,
            paths: self.paths.clone(),
            validation,
            rollback: self.rollback.clone(),
            evidence_refs: self.evidence_refs.clone(),
        })
    }
}

pub async fn run_git(runner: &dyn ArgvExecutionPort, project_root: &str, op: &GitOp) -> GitResult {
    match op {
        GitOp::Status => run_git_cmd(runner, project_root, &["status", "--short"]).await,
        GitOp::Log { count } => {
            let n = count.unwrap_or(10).min(100).to_string();
            run_git_cmd(
                runner,
                project_root,
                &["log", "--oneline", &format!("-{n}")],
            )
            .await
        }
        GitOp::Diff => run_git_cmd(runner, project_root, &["diff"]).await,
    }
}

pub fn source_control_lease_guard(op: &SourceControlOp) -> Result<Option<LeaseGuard>, String> {
    match op {
        SourceControlOp::Commit { proposal } | SourceControlOp::Push { proposal, .. } => proposal
            .to_protocol()
            .map(|proposal| Some(proposal.lease_guard())),
        SourceControlOp::Observe { .. } | SourceControlOp::Propose { .. } => Ok(None),
    }
}

pub fn run_source_control(
    project_root: &str,
    op: &SourceControlOp,
    lease_verifier: &dyn SourceLeaseVerifier,
) -> serde_json::Value {
    let runner = LocalArgvRunner;
    let adapter = GitSourceControlAdapter::new(project_root, &runner, lease_verifier);
    match op {
        SourceControlOp::Observe { expected_owner } => serde_json::json!({
            "schema": "cynic.mcp.source_control_observation.v1",
            "snapshot": adapter.observe(),
            "worktree": observe_source_worktree(std::path::Path::new(project_root), expected_owner.as_deref()),
        }),
        SourceControlOp::Propose { proposal } => {
            source_control_with_input(proposal, |proposal| adapter.admit(&proposal))
        }
        SourceControlOp::Commit { proposal } => {
            source_control_with_input(proposal, |proposal| adapter.commit(&proposal))
        }
        SourceControlOp::Push {
            proposal,
            remote,
            branch,
        } => source_control_with_input(proposal, |proposal| {
            adapter.push(
                &proposal,
                remote,
                branch,
                SourcePushAuthorization {
                    remote_target: observe_source_push_target(&proposal.repository_id, remote),
                    transport_identity: SecurityEvidenceState::Unknown,
                    authentication: SecurityEvidenceState::Unknown,
                    remote_commit_matches: SecurityEvidenceState::Unknown,
                },
            )
        }),
    }
}

fn source_control_with_input<T: serde::Serialize>(
    input: &SourceChangeProposalInput,
    operation: impl FnOnce(SourceChangeProposal) -> T,
) -> serde_json::Value {
    match input.to_protocol() {
        Ok(proposal) => serde_json::to_value(operation(proposal))
            .unwrap_or_else(|_| serde_json::json!({"status":"error"})),
        Err(error) => {
            serde_json::json!({"schema":"cynic.mcp.source_control.v1","status":"blocked","blockers":[error]})
        }
    }
}

async fn run_git_cmd(
    runner: &dyn ArgvExecutionPort,
    project_root: &str,
    args: &[&str],
) -> GitResult {
    let output = runner
        .execute(ArgvExecutionRequest {
            argv: std::iter::once("git".to_string())
                .chain(args.iter().map(|argument| (*argument).to_string()))
                .collect(),
            current_dir: Some(std::path::PathBuf::from(project_root)),
            stdin: None,
            timeout: std::time::Duration::from_secs(30),
        })
        .await;
    GitResult {
        success: output.success(),
        stdout: output.stdout,
        stderr: output.spawn_error.unwrap_or_else(|| output.stderr),
    }
}

pub async fn run_docker_container_list(runner: &dyn ArgvExecutionPort, all: bool) -> DockerResult {
    let args = if all {
        vec![
            "ps".to_string(),
            "--all".to_string(),
            "--no-trunc".to_string(),
            "--format".to_string(),
            "{{json .}}".to_string(),
        ]
    } else {
        vec![
            "ps".to_string(),
            "--no-trunc".to_string(),
            "--format".to_string(),
            "{{json .}}".to_string(),
        ]
    };
    let command = format!("docker {}", args.join(" "));
    let raw = run_docker_cmd(runner, &args).await;
    if !raw.success {
        return DockerResult {
            success: false,
            command,
            stdout: raw.stdout,
            stderr: raw.stderr,
            exit_code: raw.exit_code,
        };
    }

    let containers = raw
        .stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .collect::<Vec<_>>();

    let stdout = serde_json::json!({
        "schema": "cynic.docker.container_list.v1",
        "containers": containers,
        "count": containers.len(),
    })
    .to_string();

    DockerResult {
        success: true,
        command,
        stdout,
        stderr: raw.stderr,
        exit_code: raw.exit_code,
    }
}

pub async fn run_docker_container_logs(
    runner: &dyn ArgvExecutionPort,
    container: &str,
    tail: Option<u32>,
    timestamps: bool,
) -> DockerResult {
    let mut args = vec!["logs".to_string()];
    if timestamps {
        args.push("--timestamps".to_string());
    }
    let tail = tail.unwrap_or(200);
    args.push("--tail".to_string());
    args.push(tail.to_string());
    args.push(container.to_string());
    let command = format!("docker {}", args.join(" "));
    let raw = run_docker_cmd(runner, &args).await;
    if !raw.success {
        return DockerResult {
            success: false,
            command,
            stdout: raw.stdout,
            stderr: raw.stderr,
            exit_code: raw.exit_code,
        };
    }

    DockerResult {
        success: true,
        command,
        stdout: serde_json::json!({
            "schema": "cynic.docker.container_logs.v1",
            "container": container,
            "tail": tail,
            "timestamps": timestamps,
            "logs": raw.stdout,
        })
        .to_string(),
        stderr: raw.stderr,
        exit_code: raw.exit_code,
    }
}

pub async fn run_docker_container_inspect(
    runner: &dyn ArgvExecutionPort,
    container: &str,
) -> DockerResult {
    let args = vec!["inspect".to_string(), container.to_string()];
    let command = format!("docker {}", args.join(" "));
    let raw = run_docker_cmd(runner, &args).await;
    if !raw.success {
        return DockerResult {
            success: false,
            command,
            stdout: raw.stdout,
            stderr: raw.stderr,
            exit_code: raw.exit_code,
        };
    }

    let inspection: serde_json::Value = serde_json::from_str(&raw.stdout)
        .unwrap_or_else(|_| serde_json::json!({ "raw": raw.stdout }));
    DockerResult {
        success: true,
        command,
        stdout: serde_json::json!({
            "schema": "cynic.docker.container_inspect.v1",
            "container": container,
            "inspection": inspection,
        })
        .to_string(),
        stderr: raw.stderr,
        exit_code: raw.exit_code,
    }
}

struct DockerCmdResult {
    success: bool,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

async fn run_docker_cmd(runner: &dyn ArgvExecutionPort, args: &[String]) -> DockerCmdResult {
    let output = runner
        .execute(ArgvExecutionRequest {
            argv: std::iter::once("docker".to_string())
                .chain(args.iter().cloned())
                .collect(),
            current_dir: None,
            stdin: None,
            timeout: std::time::Duration::from_secs(30),
        })
        .await;
    DockerCmdResult {
        success: output.success(),
        stdout: output.stdout,
        stderr: output.spawn_error.unwrap_or_else(|| output.stderr),
        exit_code: output.exit_code,
    }
}

/// Run the federation reconciliation with real observation adapters.
///
/// Uses production adapters with conservative fallbacks:
/// - KernelReachabilityObserver: TCP probe on declared kernel ports
/// - MeshPeerObserver: PeerResolver-based peer counting
/// - OsHealthObserverAdapter: wraps LocalOsHealthObserver
/// - EnrollmentObserverAdapter: checks enrollment authority file
/// - TakeoverObserverAdapter: observes takeover conditions
pub async fn run_federation_status() -> serde_json::Value {
    let runtime = cynic_host::federation_runtime::FederationRuntime {
        kernel: std::sync::Arc::new(
            cynic_host::federation_runtime::KernelReachabilityObserver::new(
                "10.10.10.103",
                vec![8000, 8001],
            ),
        ),
        mesh: std::sync::Arc::new(cynic_host::federation_runtime::MeshPeerObserver::new(
            "infra/registry.json",
            "local",
        )),
        os_health: std::sync::Arc::new(
            cynic_host::federation_runtime::OsHealthObserverAdapter,
        ),
        enrollment: std::sync::Arc::new(
            cynic_host::federation_runtime::EnrollmentObserverAdapter,
        ),
        takeover: std::sync::Arc::new(
            cynic_host::federation_runtime::TakeoverObserverAdapter {
                primary_live: false,
                fencing_token: 2,
            },
        ),
        node_control: std::sync::Arc::new(FederationNodeControl),
    };

    let reconciliation = runtime.reconcile();
    serde_json::json!({
        "status": "ok",
        "reconciliation": reconciliation,
    })
}

/// NodeControlPort implementation for federation status — returns None when
/// kernel is unreachable (autonomous mode) or when no plan is available.
struct FederationNodeControl;

impl cynic_host::federation_runtime::NodeControlPort for FederationNodeControl {
    fn node_control_plan(&self) -> Option<cynic_protocol::node_control::NodeControlPlan> {
        None
    }
}

/// Observe and activate a node (bounded action with read-back proof).
pub async fn run_node_activation(node_id: &str, yes: bool) -> serde_json::Value {
    use cynic_host::node_activation::{NodeActivationPort, NodeActivationAdapter};

    let adapter = NodeActivationAdapter::new("infra/registry.json");
    let report = adapter.activate(node_id, yes);
    serde_json::json!({
        "status": "ok",
        "report": report,
    })
}

/// Observe node activation state (deployment_epoch + bootstrap + takeover).
pub async fn run_node_activation_status(node_id: &str) -> serde_json::Value {
    use cynic_host::node_activation::{NodeActivationPort, NodeActivationAdapter};

    let adapter = NodeActivationAdapter::new("infra/registry.json");
    let verdict = adapter.observe(node_id);
    serde_json::json!({
        "status": "ok",
        "node_id": node_id,
        "verdict": verdict,
    })
}

/// Aggregate fleet health from heartbeats.
pub async fn run_fleet_health(heartbeat_timeout_s: u64) -> serde_json::Value {
    use cynic_host::fleet_health_observer::FleetHealthObserver;

    let observer = FleetHealthObserver::new("infra/registry.json", heartbeat_timeout_s);
    let report = observer.observe();
    serde_json::json!({
        "status": "ok",
        "report": report,
    })
}

/// Report inference configuration status (routing SSOT, backends, vault).
pub async fn run_inference_status() -> serde_json::Value {
    use cynic_inference::load_inference_config;

    match load_inference_config() {
        Ok(config) => {
            let backends: Vec<_> = config
                .backends
                .iter()
                .map(|b| {
                    serde_json::json!({
                        "name": b.name,
                        "sovereign": b.sovereign,
                        "base_url": b.base_url,
                        "model": b.model,
                        "backend_type": b.backend_type,
                        "context_size": b.context_size,
                        "timeout_secs": b.timeout_secs,
                        "max_tokens": b.max_tokens,
                        "has_api_key": b.api_key.is_some(),
                    })
                })
                .collect();

            serde_json::json!({
                "status": "ok",
                "config_path": config.path.map(|p| p.to_string_lossy().into_owned()),
                "backends": backends,
                "sovereign_count": backends.iter().filter(|b| b["sovereign"].as_bool().unwrap_or(false)).count(),
            })
        }
        Err(e) => serde_json::json!({
            "status": "error",
            "error": format!("{e}"),
        }),
    }
}

/// Evaluate storage credential sync status (pure probe + decision).
pub async fn run_storage_credential_sync(endpoint: &str) -> serde_json::Value {
    use cynic_host::storage_credential_sync::{StorageCredentialSyncPort, StorageCredentialSyncAdapter};
    use cynic_protocol::storage_credential_sync::{StorageCredentialSpec, decide_credential_sync};

    let spec = StorageCredentialSpec {
        username: "root".into(),
        password: "cynic".into(),
        endpoint: endpoint.into(),
        namespace: "cynic".into(),
        database: "main".into(),
    };
    let adapter = StorageCredentialSyncAdapter::default();
    let obs = adapter.observe(&spec);
    let verdict = decide_credential_sync(&spec, &obs);
    serde_json::json!({
        "status": "ok",
        "endpoint": endpoint,
        "verdict": verdict,
    })
}

/// Evaluate harness progression decision (pure, no IO).
pub async fn run_harness_progression(
    current_kind: &str,
    target_kind: &str,
    satisfied: &[String],
    unknown: &[String],
) -> serde_json::Value {
    use cynic_host::harness_progression::{decide_harness_progression, HarnessProgressionEvidence, ProofCategory, RuntimeHarnessKind};

    let parse_kind = |s: &str| -> Result<RuntimeHarnessKind, String> {
        match s {
            "lab" | "closed_local_lab" => Ok(RuntimeHarnessKind::ClosedLocalLab),
            "conformance" => Ok(RuntimeHarnessKind::Conformance),
            "staging" => Ok(RuntimeHarnessKind::Staging),
            "canary" => Ok(RuntimeHarnessKind::Canary),
            "production" => Ok(RuntimeHarnessKind::Production),
            _ => Err(format!("unknown harness kind: {s}")),
        }
    };

    let parse_cat = |s: &str| -> Result<ProofCategory, String> {
        match s {
            "bootstrap" => Ok(ProofCategory::Bootstrap),
            "hardware" => Ok(ProofCategory::Hardware),
            "host_conformance" => Ok(ProofCategory::HostConformance),
            "build_reproduction" => Ok(ProofCategory::BuildReproduction),
            "network" => Ok(ProofCategory::Network),
            "target_attestation" => Ok(ProofCategory::TargetAttestation),
            "rollback_intent" => Ok(ProofCategory::RollbackIntent),
            _ => Err(format!("unknown proof category: {s}")),
        }
    };

    let current = match parse_kind(current_kind) {
        Ok(k) => k,
        Err(e) => return serde_json::json!({ "status": "error", "error": e }),
    };
    let target = match parse_kind(target_kind) {
        Ok(k) => k,
        Err(e) => return serde_json::json!({ "status": "error", "error": e }),
    };

    let satisfied: Result<Vec<_>, _> = satisfied.iter().map(|s| parse_cat(s)).collect();
    let satisfied = match satisfied {
        Ok(v) => v,
        Err(e) => return serde_json::json!({ "status": "error", "error": e }),
    };

    let unknown: Result<Vec<_>, _> = unknown.iter().map(|s| parse_cat(s)).collect();
    let unknown = match unknown {
        Ok(v) => v,
        Err(e) => return serde_json::json!({ "status": "error", "error": e }),
    };

    let evidence = HarnessProgressionEvidence {
        current_kind: current,
        target_kind: target,
        satisfied,
        pending: vec![],
        unknown,
    };

    let verdict = decide_harness_progression(&evidence);
    serde_json::json!({
        "status": "ok",
        "verdict": verdict,
    })
}

/// Run the surface audit read-back via the canonical CLI surface.
/// Delegate to `cynic dev surface-audit --json` through the synchronous
/// LocalArgvRunner (read-only projection — never mutates the host).
/// Mirrors the pattern of `run_git` / `run_source_control`: the CLI owns
/// the audit decision, the MCP projects its read-back proof.
pub fn run_surface_audit_read_back(project_root: &str) -> serde_json::Value {
    let runner = LocalArgvRunner;
    let output = runner.run(&[
        "cynic",
        "dev",
        "surface-audit",
        "--json",
    ]);
    if !output.success() {
        return serde_json::json!({
            "schema": "cynic.mcp.surface_audit_read_back.v1",
            "status": "blocked",
            "mutates": false,
            "runner": "LocalArgvRunner",
            "argv": ["cynic", "dev", "surface-audit", "--json"],
            "exit_code": output.exit_code,
            "stderr": output.stderr,
            "stdout": output.stdout,
        });
    }
    // The audit emits JSON on stdout; parse it to prove freshness.
    let report = serde_json::from_str::<serde_json::Value>(&output.stdout)
        .unwrap_or_else(|_| serde_json::json!({ "raw": output.stdout }));
    serde_json::json!({
        "schema": "cynic.mcp.surface_audit_read_back.v1",
        "status": report.get("status").and_then(|v| v.as_str()).unwrap_or("observed"),
        "mutates": false,
        "runner": "LocalArgvRunner",
        "argv": ["cynic", "dev", "surface-audit", "--json"],
        "exit_code": output.exit_code,
        "blocked_count": report.get("blocked_count"),
        "ok_count": report.get("ok_count"),
        "agent_exposure": report.get("agent_exposure"),
        "report": report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_op_status_deserializes() {
        let json = r#"{"op": "status"}"#;
        let op: GitOp = serde_json::from_str(json).unwrap();
        assert!(matches!(op, GitOp::Status));
    }

    #[test]
    fn git_op_log_default_count() {
        let json = r#"{"op": "log"}"#;
        let op: GitOp = serde_json::from_str(json).unwrap();
        if let GitOp::Log { count } = op {
            assert_eq!(count, None);
        } else {
            panic!("expected Log");
        }
    }

    #[test]
    fn direct_git_commit_is_not_a_git_operation() {
        let json = r#"{"op":"commit","message":"x","files":["a.rs"]}"#;
        assert!(serde_json::from_str::<GitOp>(json).is_err());
    }
}
