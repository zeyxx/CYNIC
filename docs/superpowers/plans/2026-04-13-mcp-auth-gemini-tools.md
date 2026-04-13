# MCP Auth + Gemini Build Tools Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Secure the MCP surface with Bearer auth (RC1-1) and add two safe build tools (`cynic_validate`, `cynic_git`) so Gemini can self-validate and commit code despite snap sandbox restrictions.

**Architecture:** MCP auth is session-based: the client provides the API key in an `cynic_auth` tool call at session start; the server stores `authenticated: bool` on the CynicMcp instance. Sensitive tools check this flag. Two new tools expose predefined operations (no shell injection surface) — `cynic_validate` runs the cargo build/test/clippy pipeline, `cynic_git` handles status/log/diff/commit with typed parameters.

**Tech Stack:** Rust (rmcp, tokio::process, schemars, serde)

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Modify | `cynic-kernel/src/api/mcp/mod.rs` | Add `authenticated` field, `cynic_auth` tool, auth checks on sensitive tools |
| Create | `cynic-kernel/src/api/mcp/build_tools.rs` | `cynic_validate` and `cynic_git` implementations (isolated from main MCP file) |
| Modify | `cynic-kernel/src/api/mcp/mod.rs` | Import + wire build_tools |

---

### Task 1: MCP Auth — `cynic_auth` tool + session flag

**Files:**
- Modify: `cynic-kernel/src/api/mcp/mod.rs`

- [ ] **Step 1: Add `authenticated` field to CynicMcp**

Add to the `CynicMcp` struct (after `bg_semaphore`):

```rust
/// RC1-1: Session authentication state. Set to true after successful cynic_auth call.
/// Uses interior mutability — MCP handler methods take &self, not &mut self.
authenticated: Arc<std::sync::atomic::AtomicBool>,
```

Initialize in `new()`:
```rust
authenticated: Arc::new(std::sync::atomic::AtomicBool::new(false)),
```

- [ ] **Step 2: Add AuthParams and cynic_auth tool**

Add to params section:
```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthParams {
    /// API key — must match the kernel's CYNIC_API_KEY
    pub api_key: String,
    /// Agent identity for audit trail
    pub agent_id: Option<String>,
}
```

Add tool (place it FIRST, before cynic_judge):
```rust
#[tool(
    name = "cynic_auth",
    description = "Authenticate this MCP session. Required before calling sensitive tools (judge, observe, validate, git, coord). Pass the CYNIC_API_KEY. Call once per session."
)]
async fn cynic_auth(
    &self,
    params: Parameters<AuthParams>,
) -> Result<CallToolResult, McpError> {
    let p = params.0;
    let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

    let expected = std::env::var("CYNIC_API_KEY").unwrap_or_default();
    if expected.is_empty() {
        return Err(McpError::internal_error(
            "Kernel has no CYNIC_API_KEY configured", None,
        ));
    }

    if p.api_key != expected {
        tracing::warn!(agent_id, "MCP auth failed — invalid key");
        return Err(McpError::new(
            rmcp::model::ErrorCode(-32000),
            "Authentication failed — invalid API key",
            None,
        ));
    }

    self.authenticated.store(true, std::sync::atomic::Ordering::Relaxed);
    tracing::info!(agent_id, "MCP session authenticated");

    Ok(CallToolResult::success(vec![Content::text(
        r#"{"authenticated": true}"#,
    )]))
}
```

- [ ] **Step 3: Add `require_auth` helper**

```rust
/// RC1-1: Check session authentication. Returns Err if not authenticated.
fn require_auth(&self) -> Result<(), McpError> {
    if !self.authenticated.load(std::sync::atomic::Ordering::Relaxed) {
        return Err(McpError::new(
            rmcp::model::ErrorCode(-32000),
            "Not authenticated — call cynic_auth first",
            None,
        ));
    }
    Ok(())
}
```

- [ ] **Step 4: Add auth checks to sensitive tools**

Add `self.require_auth()?;` as the FIRST line of these tool methods (before rate_limit.check):
- `cynic_judge`
- `cynic_infer`
- `cynic_observe`
- `cynic_coord_register`
- `cynic_coord_claim`
- `cynic_coord_claim_batch`
- `cynic_coord_release`

Leave these PUBLIC (no auth required):
- `cynic_auth` (the auth tool itself)
- `cynic_health` (monitoring must work without auth)
- `cynic_verdicts` (read-only)
- `cynic_crystals` (read-only)
- `cynic_audit_query` (read-only)
- `cynic_coord_who` (read-only visibility)

- [ ] **Step 5: Build and test**

```bash
export RUST_MIN_STACK=16777216
cargo build -p cynic-kernel --tests
cargo test -p cynic-kernel -- --nocapture 2>&1 | tail -5
```

Expected: compiles, all existing tests pass (test CynicMcp instances never call require_auth since they test specific tools directly).

Note: Existing MCP tests construct CynicMcp without authentication. Tests that call sensitive tools (judge, observe, coord) will now fail because `require_auth()` rejects them. Fix: in each `test_mcp()` helper function (there are TWO — search for `fn test_mcp`), add `mcp.authenticated.store(true, std::sync::atomic::Ordering::Relaxed);` after constructing the CynicMcp instance so all existing tests remain authenticated.

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/api/mcp/mod.rs
git commit -m "feat(mcp): RC1-1 session auth — cynic_auth tool + require_auth gate on sensitive tools"
```

---

### Task 2: `cynic_validate` — build/test/clippy pipeline

**Files:**
- Create: `cynic-kernel/src/api/mcp/build_tools.rs`
- Modify: `cynic-kernel/src/api/mcp/mod.rs` (import)

Note: `mod.rs` is already 1035 lines. Build tools are a separate concern — put them in their own file. The tools are free functions that take the project root path and return structured results. The MCP tool methods in mod.rs call these functions.

- [ ] **Step 1: Create build_tools.rs with ValidateResult**

```rust
//! Build tools — predefined operations for Gemini and other sandboxed agents.
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
```

- [ ] **Step 2: Implement run_validate**

```rust
/// Run the full validation pipeline: cargo build --tests + clippy + test.
/// RUST_MIN_STACK=16777216 is set automatically (A1 infrastructure debt).
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
            passed: false, build_ok: false, clippy_ok: false, test_ok: false,
            stdout: all_stdout, stderr: all_stderr,
            duration_ms: start.elapsed().as_millis() as u64,
        };
    }

    // Step 2: cargo clippy
    let clippy = run_cargo(project_root, &["clippy", "--all", "--", "-D", "warnings"]).await;
    all_stdout.push_str(&format!("=== CLIPPY ===\n{}\n", clippy.stdout));
    all_stderr.push_str(&clippy.stderr);
    if !clippy.success {
        return ValidateResult {
            passed: false, build_ok: true, clippy_ok: false, test_ok: false,
            stdout: all_stdout, stderr: all_stderr,
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
            .env("RUST_MIN_STACK", "16777216")
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
        Err(_) => CmdResult {
            success: false,
            stdout: String::new(),
            stderr: "Cargo timed out (300s)".to_string(),
        },
    }
}
```

- [ ] **Step 3: Implement run_git**

```rust
/// Git operations — each maps to a specific git command with typed args.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "op")]
pub enum GitOp {
    /// git status --short
    #[serde(rename = "status")]
    Status,
    /// git log --oneline -N
    #[serde(rename = "log")]
    Log { count: Option<u32> },
    /// git diff (staged + unstaged)
    #[serde(rename = "diff")]
    Diff,
    /// git add <files> + git commit -m <message>
    #[serde(rename = "commit")]
    Commit { message: String, files: Vec<String> },
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
                if f.contains("..") || f.starts_with('/') || f.contains('`') || f.contains('$') || f.contains(';') {
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
        Err(_) => GitResult {
            success: false,
            stdout: String::new(),
            stderr: "Git timed out (30s)".to_string(),
        },
    }
}
```

- [ ] **Step 4: Add unit tests**

```rust
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
```

- [ ] **Step 5: Build and run tests**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel build_tools -- --nocapture
```

Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add cynic-kernel/src/api/mcp/build_tools.rs
git commit -m "feat(mcp): build_tools module — cynic_validate + cynic_git with typed ops, no shell injection"
```

---

### Task 3: Wire build tools into MCP server

**Files:**
- Modify: `cynic-kernel/src/api/mcp/mod.rs`

- [ ] **Step 1: Add mod declaration and params**

At top of `mod.rs`, add after existing module structure (or if there's no existing submodule, add before the MCP Server section):

```rust
pub mod build_tools;
```

Note: `mod.rs` is the module root for `cynic-kernel/src/api/mcp/`. If it's currently a single file (not a directory), it needs to be converted to a directory module first: rename `mcp/mod.rs` stays as is since it's already `api/mcp/mod.rs`. Just create `build_tools.rs` next to it.

Add params:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateParams {
    /// Agent identity for audit trail
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GitParams {
    /// Git operation to perform (tagged enum: {"op": "status"} or {"op": "commit", "message": "...", "files": [...]})
    pub op: build_tools::GitOp,
    /// Agent identity for audit trail
    pub agent_id: Option<String>,
}
```

- [ ] **Step 2: Add project_root field to CynicMcp**

```rust
/// Project root path — for build tools (validate, git).
project_root: String,
```

Add parameter to `new()` and pass through. In `main.rs` where CynicMcp::new() is called, pass `project_root.display().to_string()`.

- [ ] **Step 3: Add cynic_validate tool**

```rust
#[tool(
    name = "cynic_validate",
    description = "Run the full validation pipeline: cargo build --tests + cargo clippy + cargo test. Returns pass/fail with stdout/stderr. Requires authentication. Takes ~2-5 minutes."
)]
async fn cynic_validate(
    &self,
    params: Parameters<ValidateParams>,
) -> Result<CallToolResult, McpError> {
    self.require_auth()?;
    self.rate_limit.check_other()?;
    let agent_id = params.0.agent_id.unwrap_or_else(|| "unknown".into());

    tracing::info!(agent_id, "cynic_validate started");
    let result = build_tools::run_validate(&self.project_root).await;

    self.audit(
        "cynic_validate",
        &agent_id,
        &serde_json::json!({
            "passed": result.passed,
            "build_ok": result.build_ok,
            "clippy_ok": result.clippy_ok,
            "test_ok": result.test_ok,
            "duration_ms": result.duration_ms,
        }),
    )
    .await;

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&result)
            .unwrap_or_else(|_| r#"{"error":"serialize failed"}"#.into()),
    )]))
}
```

- [ ] **Step 4: Add cynic_git tool**

```rust
#[tool(
    name = "cynic_git",
    description = "Git operations: status, log, diff, commit. For commit: provide message + file list. No push (deploy decision is human). Requires authentication."
)]
async fn cynic_git(
    &self,
    params: Parameters<GitParams>,
) -> Result<CallToolResult, McpError> {
    self.require_auth()?;
    self.rate_limit.check_other()?;
    let p = params.0;
    let agent_id = p.agent_id.unwrap_or_else(|| "unknown".into());

    let result = build_tools::run_git(&self.project_root, &p.op).await;

    self.audit(
        "cynic_git",
        &agent_id,
        &serde_json::json!({
            "op": format!("{:?}", p.op),
            "success": result.success,
        }),
    )
    .await;

    Ok(CallToolResult::success(vec![Content::text(
        serde_json::to_string(&result)
            .unwrap_or_else(|_| r#"{"error":"serialize failed"}"#.into()),
    )]))
}
```

- [ ] **Step 5: Update CynicMcp::new() in main.rs**

Find where `CynicMcp::new(...)` is called in `main.rs` and add `project_root.display().to_string()` as a parameter. **Also update BOTH `test_mcp()` helper functions** in the `#[cfg(test)]` module at the bottom of `mod.rs` — pass `"/tmp".to_string()` as the `project_root` argument.

- [ ] **Step 6: Build and run full test suite**

```bash
export RUST_MIN_STACK=16777216
cargo build -p cynic-kernel --tests
cargo test -p cynic-kernel -- --nocapture 2>&1 | tail -5
```

Expected: all tests pass

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/api/mcp/mod.rs cynic-kernel/src/api/mcp/build_tools.rs cynic-kernel/src/main.rs
git commit -m "feat(mcp): wire cynic_validate + cynic_git tools into MCP server"
```

---

### Task 4: Update Gemini MCP config

**Files:**
- Modify: `~/.gemini/.mcp.json`

- [ ] **Step 1: Add CYNIC_API_KEY to Gemini's MCP env**

The current `.mcp.json` already has the key in env. Verify Gemini can call `cynic_auth` with it. No code change needed — just document that Gemini should call `cynic_auth` at session start.

- [ ] **Step 2: Update GEMINI.md with auth protocol**

Add to GEMINI.md after the build commands section:

```markdown
## MCP Authentication

At session start, authenticate with the CYNIC kernel:
```
cynic_auth(api_key=$CYNIC_API_KEY, agent_id="gemini-<session>")
```

Then use `cynic_validate` for build verification and `cynic_git` for git operations.
```

- [ ] **Step 3: Commit**

```bash
git add GEMINI.md
git commit -m "docs(gemini): MCP auth protocol + build tools usage"
```

---

### Task 5: Clippy + full validation + E2E test

- [ ] **Step 1: Run clippy**

```bash
export RUST_MIN_STACK=16777216
cargo clippy -p cynic-kernel --all -- -D warnings
```

Expected: 0 warnings

- [ ] **Step 2: Run full test suite**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel
```

Expected: all tests pass

- [ ] **Step 3: Build release + deploy**

```bash
export RUST_MIN_STACK=16777216
cargo build -p cynic-kernel --release
mv ~/bin/cynic-kernel ~/bin/cynic-kernel.bak
cp target/release/cynic-kernel ~/bin/cynic-kernel
systemctl --user restart cynic-kernel
sleep 3
source ~/.cynic-env && curl -s "${CYNIC_REST_ADDR}/health"
```

Expected: sovereign

- [ ] **Step 4: E2E — verify auth rejection**

Using the MCP tools (via Gemini or curl-equivalent), verify:
- `cynic_judge` without prior `cynic_auth` → rejected with "Not authenticated"
- `cynic_health` without auth → still works (public)
- `cynic_auth` with wrong key → rejected
- `cynic_auth` with correct key → authenticated

- [ ] **Step 5: E2E — verify build tools**

After authentication:
- `cynic_validate` → returns pass/fail with build/clippy/test breakdown
- `cynic_git(op=status)` → returns git status
- `cynic_git(op=log, count=3)` → returns last 3 commits
