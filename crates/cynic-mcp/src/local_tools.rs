//! Local tools — predefined filesystem operations for sandboxed agents.
//! No shell injection: each operation constructs Command with typed args.
//! Auth is checked by the MCP tool layer, not here.
//!
//! K18 compliance: every Command that runs inside a timeout uses
//! `kill_on_drop(true)` + `spawn()` + `wait_with_output()`.
//! Using `.output()` directly inside `timeout()` leaves zombie processes
//! when the timeout fires (the child survives SIGTERM and pollutes the cgroup).

use std::time::Instant;

/// Result of the canonical repository validation gate (`make check`).
///
/// Authority: `AGENTS.md`, `contracts/foundation.json` evolutive_policies.build.
/// This intentionally does **not** reimplement a parallel fmt/build/clippy/test
/// stack — agents and operators share one living gate.
#[derive(Debug, serde::Serialize)]
pub struct ValidateResult {
    pub schema: &'static str,
    pub gate: &'static str,
    pub passed: bool,
    pub authority_json_ok: bool,
    pub cargo_check_ok: bool,
    pub surface_audit_ok: bool,
    pub exit_code: Option<i32>,
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
    #[serde(rename = "commit")]
    Commit { message: String, files: Vec<String> },
}

pub async fn run_validate(project_root: &str) -> ValidateResult {
    let start = Instant::now();
    let elapsed_ms = |start: Instant| start.elapsed().as_millis() as u64;

    // Single canonical gate. Make injects CYNIC_ALLOW_LOCAL_DEV_BUILD, stack,
    // offline cargo mode, and surface-audit. Do not invent a second stack here.
    let result = run_make_check(project_root).await;
    let (authority_json_ok, cargo_check_ok, surface_audit_ok) =
        infer_make_check_steps(&result.stdout, &result.stderr, result.success);

    ValidateResult {
        schema: "cynic.mcp.validate.v1",
        gate: "make check",
        passed: result.success,
        authority_json_ok,
        cargo_check_ok,
        surface_audit_ok,
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
        duration_ms: elapsed_ms(start),
    }
}

/// Best-effort step inference from make check logs for agent-readable reports.
fn infer_make_check_steps(stdout: &str, stderr: &str, passed: bool) -> (bool, bool, bool) {
    if passed {
        return (true, true, true);
    }
    let combined = format!("{stdout}\n{stderr}");
    let cargo_started = combined.contains("cargo check core crates");
    let audit_started = combined.contains("auditing canonical CLI surfaces");
    // Fail-closed: a later stage starting means earlier stages succeeded.
    let authority_json_ok = cargo_started || audit_started;
    let cargo_check_ok = audit_started;
    let surface_audit_ok = false;
    (authority_json_ok, cargo_check_ok, surface_audit_ok)
}

struct CmdResult {
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

async fn run_make_check(project_root: &str) -> CmdResult {
    let child = tokio::process::Command::new("make")
        .arg("check")
        .current_dir(project_root)
        // Defense in depth: Makefile also sets this for nested cargo.
        .env("CYNIC_ALLOW_LOCAL_DEV_BUILD", "1")
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let result = match child {
        Ok(c) => {
            tokio::time::timeout(std::time::Duration::from_secs(900), c.wait_with_output()).await
        }
        Err(e) => {
            return CmdResult {
                success: false,
                exit_code: None,
                stdout: String::new(),
                stderr: format!(
                    "Failed to spawn `make check` (canonical gate). Install make or run from a checkout with Makefile: {e}"
                ),
            };
        }
    };

    match result {
        Ok(Ok(output)) => CmdResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Ok(Err(e)) => CmdResult {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: format!("Failed to run make check: {e}"),
        },
        Err(_elapsed) => CmdResult {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: "make check timed out (900s)".to_string(),
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
    let child = tokio::process::Command::new("git")
        .args(args)
        .current_dir(project_root)
        .kill_on_drop(true) // K18: timed-out children must not become zombies
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let result = match child {
        Ok(c) => {
            tokio::time::timeout(std::time::Duration::from_secs(30), c.wait_with_output()).await
        }
        Err(e) => {
            return GitResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Failed to spawn git: {e}"),
            };
        }
    };

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

pub async fn run_docker_container_list(all: bool) -> DockerResult {
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
    let raw = run_docker_cmd(&args).await;
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
    let raw = run_docker_cmd(&args).await;
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

pub async fn run_docker_container_inspect(container: &str) -> DockerResult {
    let args = vec!["inspect".to_string(), container.to_string()];
    let command = format!("docker {}", args.join(" "));
    let raw = run_docker_cmd(&args).await;
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

async fn run_docker_cmd(args: &[String]) -> DockerCmdResult {
    let child = tokio::process::Command::new("docker")
        .args(args)
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    let result = match child {
        Ok(c) => {
            tokio::time::timeout(std::time::Duration::from_secs(30), c.wait_with_output()).await
        }
        Err(e) => {
            return DockerCmdResult {
                success: false,
                stdout: String::new(),
                stderr: format!("Failed to spawn docker: {e}"),
                exit_code: None,
            };
        }
    };

    match result {
        Ok(Ok(output)) => DockerCmdResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
        },
        Ok(Err(e)) => DockerCmdResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to read docker output: {e}"),
            exit_code: None,
        },
        Err(_elapsed) => DockerCmdResult {
            success: false,
            stdout: String::new(),
            stderr: "Docker timed out (30s)".to_string(),
            exit_code: None,
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

    #[test]
    fn infer_steps_marks_all_ok_when_gate_passes() {
        let (a, c, s) = infer_make_check_steps("ok", "", true);
        assert!(a && c && s);
    }

    #[test]
    fn infer_steps_fails_closed_before_cargo() {
        let (a, c, s) = infer_make_check_steps(
            "==> checking foundation, runtime authority, and registry\n",
            "jq: parse error",
            false,
        );
        assert!(!a);
        assert!(!c);
        assert!(!s);
    }

    #[test]
    fn infer_steps_authority_ok_when_cargo_started() {
        let (a, c, s) = infer_make_check_steps(
            "==> checking foundation, runtime authority, and registry\n==> cargo check core crates\n",
            "error: could not compile",
            false,
        );
        assert!(a);
        assert!(!c);
        assert!(!s);
    }
}
