use super::shared::GitResult;
use cynic_host::ports::argv_execution::{ArgvExecutionPort, ArgvExecutionRequest};

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
