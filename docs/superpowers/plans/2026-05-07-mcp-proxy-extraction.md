# MCP Proxy Extraction — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract the MCP-to-REST proxy into a standalone `cynic-mcp` crate (library + binary).

**Architecture:** New workspace member `cynic-mcp/` with ~9 deps. Proxy.rs moves verbatim with import rewrites. Local tools (validate, git) move to `local_tools.rs`. Param types duplicated from kernel (proxy's own contract). Kernel keeps `--mcp` as deprecated path.

**Tech Stack:** Rust, rmcp 1.2, reqwest 0.12, tokio, serde, schemars

**Spec:** `docs/superpowers/specs/2026-05-07-mcp-proxy-extraction-design.md`

---

## File Structure

```
cynic-mcp/
  Cargo.toml          — workspace member, 9 deps
  build.rs            — git version injection (CYNIC_VERSION)
  src/
    lib.rs            — pub mod proxy, types, local_tools
    types.rs          — param structs + McpRateLimit + validate_agent_id
    proxy.rs          — CynicMcpProxy (moved from kernel, imports rewritten)
    local_tools.rs    — run_validate + run_git (moved from kernel build_tools.rs)
    main.rs           — binary entry point

Modified:
  Cargo.toml          — add cynic-mcp to workspace members
  cynic-kernel/src/main.rs — deprecation warning on --mcp
  .claude/settings.json — MCP server command → cynic-mcp
```

---

### Task 1: Scaffold cynic-mcp crate

**Files:**
- Create: `cynic-mcp/Cargo.toml`
- Create: `cynic-mcp/build.rs`
- Create: `cynic-mcp/src/lib.rs`
- Create: `cynic-mcp/src/main.rs` (stub)
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create Cargo.toml**

Create `cynic-mcp/Cargo.toml`:
```toml
[package]
name = "cynic-mcp"
version = "0.1.0"
edition = "2024"

[dependencies]
rmcp = { version = "1.2", features = ["server", "transport-io", "macros"] }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
chrono = "0.4"

[lints]
workspace = true
```

- [ ] **Step 2: Create build.rs**

Create `cynic-mcp/build.rs` — extract only the CYNIC_VERSION injection from `cynic-kernel/build.rs`:
```rust
fn git_stdout(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn main() {
    for git_path in [
        git_stdout(&["rev-parse", "--git-path", "HEAD"]),
        git_stdout(&["rev-parse", "--git-path", "index"]),
        git_stdout(&["rev-parse", "--git-path", "packed-refs"]),
    ]
    .into_iter()
    .flatten()
    {
        println!("cargo:rerun-if-changed={git_path}");
    }
    if let Some(head_ref) = git_stdout(&["symbolic-ref", "-q", "HEAD"])
        && let Some(ref_path) = git_stdout(&["rev-parse", "--git-path", &head_ref])
    {
        println!("cargo:rerun-if-changed={ref_path}");
    }
    let git_version = git_stdout(&["describe", "--tags", "--always", "--dirty"])
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=CYNIC_VERSION={git_version}");
}
```

- [ ] **Step 3: Create stub lib.rs and main.rs**

`cynic-mcp/src/lib.rs`:
```rust
pub mod local_tools;
pub mod proxy;
pub mod types;
```

`cynic-mcp/src/main.rs`:
```rust
fn main() {
    println!("cynic-mcp stub");
}
```

- [ ] **Step 4: Add to workspace**

In root `Cargo.toml`, add `"cynic-mcp"` to workspace members list.

- [ ] **Step 5: Verify scaffold compiles**

Run: `cargo check -p cynic-mcp`
Expected: errors about missing modules (types, proxy, local_tools) — that's fine, scaffold works.

Actually — create empty files first so the `pub mod` declarations resolve:
```bash
touch cynic-mcp/src/types.rs cynic-mcp/src/proxy.rs cynic-mcp/src/local_tools.rs
```
Then: `cargo check -p cynic-mcp` → should compile (empty modules).

- [ ] **Step 6: Commit**

```bash
git add cynic-mcp/ Cargo.toml
git commit -m "feat(mcp): scaffold cynic-mcp crate (empty modules)"
```

---

### Task 2: Move local_tools (validate + git)

**Files:**
- Create: `cynic-mcp/src/local_tools.rs`

Note: local_tools must come BEFORE types because `types.rs` references `crate::local_tools::GitOp`.

- [ ] **Step 1: Write local_tools.rs**

Copy from `cynic-kernel/src/api/mcp/build_tools.rs` (all 314 lines). The file is self-contained: `ValidateResult`, `GitResult`, `GitOp`, `run_validate`, `run_git`, `run_cargo`, `run_git_cmd`, plus tests.

**K18 fix required:** Both `run_cargo` and `run_git_cmd` use `tokio::process::Command` inside `tokio::time::timeout` without `kill_on_drop(true)`. Fix both:

```rust
// In run_cargo: change Command construction to:
let mut child = tokio::process::Command::new("cargo")
    .args(args)
    .current_dir(project_root)
    .env("RUST_MIN_STACK", "67108864")
    .env("RUSTFLAGS", "-C debuginfo=1")
    .kill_on_drop(true)   // K18: timed-out children must not become zombies
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn();

// Then use timeout on child.wait_with_output() instead of cmd.output()

// Same pattern for run_git_cmd:
let mut child = tokio::process::Command::new("git")
    .args(args)
    .current_dir(project_root)
    .kill_on_drop(true)   // K18
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn();
```

- [ ] **Step 2: Verify compiles**

Run: `cargo check -p cynic-mcp`
Expected: PASS

- [ ] **Step 3: Run local_tools tests**

Run: `cargo test -p cynic-mcp -- local_tools`
Expected: 6 tests pass (git_op deserializers + commit rejection tests)

- [ ] **Step 4: Commit**

```bash
git add cynic-mcp/src/local_tools.rs
git commit -m "feat(mcp): add local tools (validate + git, K18 compliant)"
```

---

### Task 3: Move types (param structs + McpRateLimit + validate_agent_id)

**Files:**
- Create: `cynic-mcp/src/types.rs`

- [ ] **Step 1: Write types.rs**

Copy from `cynic-kernel/src/api/mcp/mod.rs` lines 39-93 (McpRateLimit) and lines 109-229 (param structs). Also copy `validate_agent_id` but inline the validation (no kernel dependency):

Key changes from kernel source:
- Replace `use crate::domain::coord::validate_agent_id` with an inline implementation
- `GitParams.op` field type: `build_tools::GitOp` becomes `crate::local_tools::GitOp`
- All `pub(crate)` → `pub` (library crate, not `pub(crate)`)
- Keep all `#[derive(Debug, Deserialize, JsonSchema)]`

The `validate_agent_id` function becomes:
```rust
pub fn validate_agent_id(agent_id: &Option<String>) -> Result<(), McpError> {
    if let Some(id) = agent_id {
        if id.is_empty() || id.chars().count() > 64 {
            return Err(McpError::invalid_params("agent_id must be 1-64 characters", None));
        }
        if id.chars().any(|c| c.is_control()) {
            return Err(McpError::invalid_params("agent_id contains invalid characters", None));
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Verify compiles**

Run: `cargo check -p cynic-mcp`
Expected: PASS (types.rs compiles, references `crate::local_tools::GitOp` which exists from Task 2)

- [ ] **Step 3: Commit**

```bash
git add cynic-mcp/src/types.rs
git commit -m "feat(mcp): add param structs and rate limiter types"
```

---

### Task 4: Move proxy (CynicMcpProxy)

**Files:**
- Modify: `cynic-mcp/src/proxy.rs`

- [ ] **Step 1: Copy proxy.rs from kernel**

Copy `cynic-kernel/src/api/mcp/proxy.rs` to `cynic-mcp/src/proxy.rs`.

Apply these import rewrites:
```rust
// OLD:
use super::{
    AuditQueryParams, AuthParams, BatchClaimParams, ClaimParams, DispatchAgentTaskParams,
    GitParams, JudgeParams, ListParams, ListPendingAgentTasksParams, McpRateLimit, ObserveParams,
    RegisterParams, ReleaseParams, UpdateAgentTaskResultParams, ValidateParams, WhoParams,
    validate_agent_id,
};

// NEW:
use crate::types::{
    AuditQueryParams, AuthParams, BatchClaimParams, ClaimParams, DispatchAgentTaskParams,
    GitParams, JudgeParams, ListParams, ListPendingAgentTasksParams, McpRateLimit, ObserveParams,
    RegisterParams, ReleaseParams, UpdateAgentTaskResultParams, ValidateParams, WhoParams,
    validate_agent_id,
};
```

Apply visibility changes on `#[tool_router]` macros:
```rust
// OLD (3 occurrences):
#[tool_router(router = tool_router_forward, vis = "pub(super)")]
#[tool_router(router = tool_router_coord, vis = "pub(super)")]
#[tool_router(router = tool_router_local, vis = "pub(super)")]

// NEW:
#[tool_router(router = tool_router_forward, vis = "pub(crate)")]
#[tool_router(router = tool_router_coord, vis = "pub(crate)")]
#[tool_router(router = tool_router_local, vis = "pub(crate)")]
```

Apply local_tools call rewrites:
```rust
// OLD:
let result = super::build_tools::run_validate(&self.project_root).await;
let result = super::build_tools::run_git(&self.project_root, &params.0.op).await;

// NEW:
let result = crate::local_tools::run_validate(&self.project_root).await;
let result = crate::local_tools::run_git(&self.project_root, &params.0.op).await;
```

Change server info name:
```rust
// OLD:
Implementation::new("cynic-kernel-proxy", env!("CYNIC_VERSION"))

// NEW:
Implementation::new("cynic-mcp", env!("CYNIC_VERSION"))
```

- [ ] **Step 2: Verify compiles**

Run: `cargo check -p cynic-mcp`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add cynic-mcp/src/proxy.rs
git commit -m "feat(mcp): move CynicMcpProxy with import rewrites"
```

---

### Task 5: Write main.rs (binary entry point)

**Files:**
- Modify: `cynic-mcp/src/main.rs`

- [ ] **Step 1: Write main.rs**

Adapted from kernel's MCP early exit block (`cynic-kernel/src/main.rs` lines 57-98). Key differences:
- Always proxy mode (no conditional)
- Stdout guard before any logging
- `--version` flag support
- Stderr-only logging (JSON, env filter)

```rust
use rmcp::ServiceExt;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--version") {
        eprintln!("cynic-mcp {}", env!("CYNIC_VERSION"));
        return Ok(());
    }

    if std::env::args().any(|a| a == "--help" || a == "-h") {
        eprintln!("cynic-mcp — MCP-to-REST proxy for CYNIC kernel");
        eprintln!();
        eprintln!("USAGE: cynic-mcp");
        eprintln!();
        eprintln!("ENVIRONMENT:");
        eprintln!("  CYNIC_REST_ADDR   Kernel REST address (default: http://127.0.0.1:3030)");
        eprintln!("  CYNIC_API_KEY     Bearer token for kernel auth");
        eprintln!("  RUST_LOG          Log filter (default: cynic_mcp=info,warn)");
        return Ok(());
    }

    // Stdout guard: MCP uses stdio for JSON-RPC. All logging goes to stderr.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("cynic_mcp=info,warn"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().json().with_writer(std::io::stderr))
        .init();

    let raw_addr =
        std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| "http://127.0.0.1:3030".into());
    let rest_addr = if raw_addr.starts_with("http://") || raw_addr.starts_with("https://") {
        raw_addr
    } else {
        format!("http://{raw_addr}")
    };
    let api_key = std::env::var("CYNIC_API_KEY").unwrap_or_default();
    let project_root = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .display()
        .to_string();

    tracing::info!(rest_addr, "cynic-mcp starting — forwarding to REST kernel");

    let proxy = cynic_mcp::proxy::CynicMcpProxy::new(rest_addr, api_key, project_root);

    let shutdown = CancellationToken::new();
    // Signal handler
    {
        let sd = shutdown.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            sd.cancel();
        });
    }

    let transport = rmcp::transport::io::stdio();
    let server = proxy
        .serve(transport)
        .await
        .map_err(|e| format!("MCP proxy error: {e}"))?;

    tokio::select! {
        _ = server.waiting() => {}
        _ = shutdown.cancelled() => {
            tracing::info!("cynic-mcp shutting down");
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Verify compiles**

Run: `cargo check -p cynic-mcp`
Expected: PASS

- [ ] **Step 3: Build the binary**

Run: `cargo build -p cynic-mcp`
Expected: binary at `target/debug/cynic-mcp`

- [ ] **Step 4: Verify --version and --help**

Run: `target/debug/cynic-mcp --version`
Expected: `cynic-mcp <git-describe>`

Run: `target/debug/cynic-mcp --help`
Expected: shows USAGE + ENVIRONMENT section

- [ ] **Step 5: Commit**

```bash
git add cynic-mcp/src/main.rs
git commit -m "feat(mcp): standalone binary entry point"
```

---

### Task 6: Kernel deprecation + settings.json update

**Files:**
- Modify: `cynic-kernel/src/main.rs` (~3 lines)
- Modify: `.claude/settings.json` (MCP server command)

- [ ] **Step 1: Add deprecation warning**

In `cynic-kernel/src/main.rs`, inside the `if mcp_mode {` block (after the tracing init, before the proxy construction), add:

```rust
tracing::warn!(
    "DEPRECATED: --mcp flag will be removed. Use the standalone cynic-mcp binary instead."
);
```

- [ ] **Step 2: Update .claude/settings.json**

If `.claude/settings.json` contains an MCP server entry referencing `cynic-kernel --mcp`, update the command to point to `cynic-mcp` binary. The exact path depends on the current config — read it first, then update the command field.

- [ ] **Step 3: Verify kernel still builds**

Run: `cargo check -p cynic-kernel`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/main.rs .claude/settings.json
git commit -m "deprecate(kernel): warn on --mcp, update settings to cynic-mcp"
```

---

### Task 7: Integration verification

**Files:** None (testing only)

- [ ] **Step 1: Full workspace check**

Run: `cargo check --workspace --all-targets`
Expected: PASS

- [ ] **Step 2: Full workspace clippy**

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 3: Run all tests**

Run: `cargo test --workspace`
Expected: all pass (including cynic-mcp local_tools tests)

- [ ] **Step 4: Build release binary**

Run: `cargo build -p cynic-mcp --release`
Expected: binary at `target/release/cynic-mcp`

- [ ] **Step 5: Verify binary size is small**

Run: `ls -lh target/release/cynic-mcp`
Expected: significantly smaller than `target/release/cynic-kernel`

- [ ] **Step 6: Test against running kernel (if available)**

If kernel is running at localhost:3030:
```bash
echo '{"jsonrpc":"2.0","method":"initialize","params":{"capabilities":{}},"id":1}' | CYNIC_REST_ADDR=http://127.0.0.1:3030 target/release/cynic-mcp
```
Expected: JSON-RPC response with server info containing "cynic-mcp"

- [ ] **Step 7: Commit final state + push**

```bash
git add -A
git commit -m "feat(mcp): cynic-mcp extraction complete — standalone MCP proxy"
```

Push and open PR.
