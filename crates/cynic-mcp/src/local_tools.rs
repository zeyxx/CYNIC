//! Local tools — predefined filesystem operations for sandboxed agents.
//! No shell injection: each operation constructs Command with typed args.
//! Auth is checked by the MCP tool layer, not here.
//!
//! K18 compliance: every Command that runs inside a timeout uses
//! `kill_on_drop(true)` + `spawn()` + `wait_with_output()`.
//! Using `.output()` directly inside `timeout()` leaves zombie processes
//! when the timeout fires (the child survives SIGTERM and pollutes the cgroup).

use cynic_host::ports::argv_execution::{ArgvExecutionPort, ArgvExecutionRequest};
use cynic_host::source_control::{
    GitSourceControlAdapter, SourceControlPort, SourcePushAuthorization,
    observe_source_push_target, observe_source_worktree,
};
use cynic_host::source_lease::SourceLeaseVerifier;
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::{LeaseGuard, SecurityEvidenceState, SourceChangeProposal, SourceMatterClass};

/// Result of a git operation.
#[derive(Debug, serde::Serialize)]
pub struct GitResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
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
