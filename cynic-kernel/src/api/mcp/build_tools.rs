//! Build tools — predefined operations for sandboxed agents (Gemini snap).
//! No shell injection: each operation constructs Command with typed args.
//! Auth is checked by the MCP tool layer, not here.

use std::time::Instant;

/// Result of a full validate cycle (build + clippy + test).
#[derive(Debug, serde::Serialize)]
pub struct ValidateResult {
    pub passed: bool,
    pub build_ok: bool,
    pub clippy_ok: bool,
    pub test_ok: bool,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

/// Result of a git operation.
#[derive(Debug, serde::Serialize)]
pub struct GitResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
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
    #[serde(rename = "commit")]
    Commit { message: String, files: Vec<String> },
}

pub async fn run_validate(project_root: &str) -> ValidateResult {
    let start = Instant::now();
    let mut all_stdout = String::new();
    let mut all_stderr = String::new();

    // Step 1: cargo build --tests
    let build = run_cargo(project_root, &["build", "--tests"]).await;
    all_stdout.push_str(&format!("=== BUILD ===\n{}\n", build.stdout));
    all_stderr.push_str(&build.stderr);
    if !build.success {
        return ValidateResult {
            passed: false,
            build_ok: false,
            clippy_ok: false,
            test_ok: false,
            stdout: all_stdout,
            stderr: all_stderr,
            duration_ms: start.elapsed().as_millis() as u64,
        };
    }

    // Step 2: cargo clippy
    let clippy = run_cargo(project_root, &["clippy", "--all", "--", "-D", "warnings"]).await;
    all_stdout.push_str(&format!("=== CLIPPY ===\n{}\n", clippy.stdout));
    all_stderr.push_str(&clippy.stderr);
    if !clippy.success {
        return ValidateResult {
            passed: false,
            build_ok: true,
            clippy_ok: false,
            test_ok: false,
            stdout: all_stdout,
            stderr: all_stderr,
            duration_ms: start.elapsed().as_millis() as u64,
        };
    }

    // Step 3: cargo test
    let test = run_cargo(project_root, &["test"]).await;
    all_stdout.push_str(&format!("=== TEST ===\n{}\n", test.stdout));
    all_stderr.push_str(&test.stderr);

    ValidateResult {
        passed: test.success,
        build_ok: true,
        clippy_ok: true,
        test_ok: test.success,
        stdout: all_stdout,
        stderr: all_stderr,
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

struct CmdResult {
    success: bool,
    stdout: String,
    stderr: String,
}

async fn run_cargo(project_root: &str, args: &[&str]) -> CmdResult {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        tokio::process::Command::new("cargo")
            .args(args)
            .current_dir(project_root)
            .env("RUST_MIN_STACK", "67108864")
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => CmdResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Ok(Err(e)) => CmdResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to spawn cargo: {e}"),
        },
        Err(_elapsed) => CmdResult {
            success: false,
            stdout: String::new(),
            stderr: "Cargo timed out (300s)".to_string(),
        },
    }
}

pub async fn run_git(project_root: &str, op: &GitOp) -> GitResult {
    match op {
        GitOp::Status => run_git_cmd(project_root, &["status", "--short"]).await,
        GitOp::Log { count } => {
            let n = count.unwrap_or(10).min(100).to_string();
            run_git_cmd(project_root, &["log", "--oneline", &format!("-{n}")]).await
        }
        GitOp::Diff => run_git_cmd(project_root, &["diff"]).await,
        GitOp::Commit { message, files } => {
            if message.is_empty() || files.is_empty() {
                return GitResult {
                    success: false,
                    stdout: String::new(),
                    stderr: "commit requires non-empty message and files".into(),
                };
            }
            // Validate file paths: no ../, no absolute paths, no shell metacharacters
            for f in files {
                if f.contains("..")
                    || f.starts_with('/')
                    || f.contains('`')
                    || f.contains('$')
                    || f.contains(';')
                {
                    return GitResult {
                        success: false,
                        stdout: String::new(),
                        stderr: format!("Invalid file path: {f}"),
                    };
                }
            }
            // git add <files>
            let mut add_args = vec!["add", "--"];
            let file_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
            add_args.extend(file_refs);
            let add = run_git_cmd(project_root, &add_args).await;
            if !add.success {
                return add;
            }
            // git commit -m <message>
            run_git_cmd(project_root, &["commit", "-m", message]).await
        }
    }
}

async fn run_git_cmd(project_root: &str, args: &[&str]) -> GitResult {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new("git")
            .args(args)
            .current_dir(project_root)
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => GitResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Ok(Err(e)) => GitResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to spawn git: {e}"),
        },
        Err(_elapsed) => GitResult {
            success: false,
            stdout: String::new(),
            stderr: "Git timed out (30s)".to_string(),
        },
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
    fn git_op_commit_deserializes() {
        let json = r#"{"op": "commit", "message": "test", "files": ["a.rs"]}"#;
        let op: GitOp = serde_json::from_str(json).unwrap();
        assert!(matches!(op, GitOp::Commit { .. }));
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

    #[tokio::test]
    async fn commit_rejects_path_traversal() {
        let op = GitOp::Commit {
            message: "bad".into(),
            files: vec!["../../etc/passwd".into()],
        };
        let result = run_git("/tmp", &op).await;
        assert!(!result.success);
        assert!(result.stderr.contains("Invalid file path"));
    }

    #[tokio::test]
    async fn commit_rejects_empty_message() {
        let op = GitOp::Commit {
            message: String::new(),
            files: vec!["a.rs".into()],
        };
        let result = run_git("/tmp", &op).await;
        assert!(!result.success);
    }

    #[tokio::test]
    async fn commit_rejects_shell_metacharacters() {
        let op = GitOp::Commit {
            message: "test".into(),
            files: vec!["a.rs; rm -rf /".into()],
        };
        let result = run_git("/tmp", &op).await;
        assert!(!result.success);
        assert!(result.stderr.contains("Invalid file path"));
    }
}
