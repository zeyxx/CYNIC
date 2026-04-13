# Gemini Dog + Nightshift Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Gemini CLI as a 5th kernel-native Dog (via `CliBackend`) and spawn a nightshift loop that autonomously judges recent commits every 4 hours, producing dev-domain crystals.

**Architecture:** New `CliBackend` struct implements `ChatPort` + `BackendPort` using `tokio::process::Command` to call the `gemini` binary non-interactively. Config gets a `backend_type` field (`"openai"` default, `"cli"` for CLI backends). `spawn_nightshift_loop` in `runtime_loops.rs` runs every 4h: parses `git log --since=24h` in Rust, feeds each commit as a stimulus to `judge_pipeline(domain="dev")`, then observes patterns. A calibration corpus (10 good + 10 bad dev stimuli) provides the baseline for measuring Dog discrimination.

**Tech Stack:** Rust (tokio, async_trait, serde, tokio::process), gemini CLI 0.37.1

---

## File Map

| Action | File | Responsibility |
|--------|------|---------------|
| Create | `cynic-kernel/src/backends/cli.rs` | `CliBackend` — ChatPort + BackendPort via subprocess |
| Modify | `cynic-kernel/src/backends/mod.rs:1-3` | Add `pub mod cli;` |
| Modify | `cynic-kernel/src/infra/config.rs:96-122` | Add `backend_type` field to `BackendEntry` |
| Modify | `cynic-kernel/src/infra/config.rs:152-218` | Branch on `backend_type` in `load_backends()` |
| Modify | `cynic-kernel/src/main.rs:225-301` | Branch on config to instantiate `CliBackend` vs `OpenAiCompatBackend` |
| Create | `cynic-kernel/src/infra/tasks/nightshift.rs` | `spawn_nightshift_loop` + `git_commits_since` + `judge_commit` |
| Modify | `cynic-kernel/src/infra/tasks.rs:8,24-27` | Add `mod nightshift;` + re-export `spawn_nightshift_loop` |
| Modify | `cynic-kernel/src/infra/task_health.rs` | Add `nightshift` field + `touch_nightshift()` + `NIGHTSHIFT` contract |
| Modify | `cynic-kernel/src/domain/constants.rs` | Add `NIGHTSHIFT_INTERVAL` (4h) + `NIGHTSHIFT_TIMEOUT` (5min) |
| Modify | `cynic-kernel/src/main.rs` (Ring 3 area) | Spawn nightshift loop |
| Modify | `~/.config/cynic/backends.toml` | Add `gemini-2.5-flash` CLI entry |
| Create | `cynic-kernel/domains/dev.md` | Dev domain prompt (axiom criteria for code/commits) |
| Create | `docs/domains/calibration.md` | 10 good + 10 bad dev stimuli for Dog discrimination baseline (NOT in domains/ to avoid auto-load as domain prompt) |

---

### Task 1: CliBackend — ChatPort + BackendPort

**Files:**
- Create: `cynic-kernel/src/backends/cli.rs`
- Modify: `cynic-kernel/src/backends/mod.rs`

- [ ] **Step 1: Write the test for CliBackend::name()**

In `cynic-kernel/src/backends/cli.rs`, at the bottom in a `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::inference::BackendPort;

    #[test]
    fn name_returns_configured_name() {
        let backend = CliBackend::new("gemini-2.5-flash", "/usr/bin/gemini", 60);
        assert_eq!(backend.name(), "gemini-2.5-flash");
    }
}
```

- [ ] **Step 2: Write the CliBackend struct + BackendPort impl**

```rust
//! CliBackend — adapter for CLI-based LLM tools (e.g. gemini-cli).
//! Implements BackendPort (health via --version) and ChatPort (via --prompt).
//! Spawns the binary as a subprocess per call — no persistent connection.

use crate::domain::chat::{ChatError, ChatPort, ChatResponse, InferenceProfile};
use crate::domain::inference::{BackendPort, BackendStatus};
use async_trait::async_trait;
use std::time::Duration;

#[derive(Debug)]
pub struct CliBackend {
    name: String,
    binary: String,
    timeout: Duration,
}

impl CliBackend {
    pub fn new(name: &str, binary: &str, timeout_secs: u64) -> Self {
        Self {
            name: name.to_string(),
            binary: binary.to_string(),
            timeout: Duration::from_secs(timeout_secs),
        }
    }
}

#[async_trait]
impl BackendPort for CliBackend {
    fn name(&self) -> &str {
        &self.name
    }

    async fn health(&self) -> BackendStatus {
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::process::Command::new(&self.binary)
                .arg("--version")
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) if output.status.success() => BackendStatus::Healthy,
            Ok(Ok(_)) => BackendStatus::Degraded { latency_ms: 0.0 },
            _ => BackendStatus::Critical,
        }
    }
}
```

- [ ] **Step 3: Run test to verify name works**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel backends::cli::tests::name_returns_configured_name -- --exact
```

Expected: PASS

- [ ] **Step 4: Write the ChatPort impl test**

```rust
#[tokio::test]
async fn chat_returns_text_from_cli_stdout() {
    // Skip if gemini not installed
    let which = tokio::process::Command::new("which")
        .arg("gemini")
        .output()
        .await;
    if which.is_err() || !which.unwrap().status.success() {
        eprintln!("SKIP: gemini CLI not found");
        return;
    }

    let backend = CliBackend::new("gemini-test", "gemini", 30);
    let resp = backend
        .chat("You are a test.", "Reply with exactly: PONG", InferenceProfile::Scoring, None)
        .await;
    assert!(resp.is_ok(), "gemini CLI should return a response");
    let text = resp.unwrap().text;
    assert!(!text.is_empty(), "response should not be empty");
}
```

- [ ] **Step 5: Write the ChatPort impl**

```rust
#[async_trait]
impl ChatPort for CliBackend {
    async fn chat(
        &self,
        system: &str,
        user: &str,
        _profile: InferenceProfile,
        _request_id: Option<&str>,
    ) -> Result<ChatResponse, ChatError> {
        // Build combined prompt — gemini CLI has no system/user separation
        let prompt = if system.is_empty() {
            user.to_string()
        } else {
            format!("{system}\n\n{user}")
        };

        let result = tokio::time::timeout(
            self.timeout,
            tokio::process::Command::new(&self.binary)
                .arg("--prompt")
                .arg(&prompt)
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) if output.status.success() => {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if text.is_empty() {
                    return Err(ChatError::Protocol(format!(
                        "{}: empty stdout",
                        self.name
                    )));
                }
                Ok(ChatResponse {
                    text,
                    prompt_tokens: 0,
                    completion_tokens: 0,
                })
            }
            Ok(Ok(output)) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(ChatError::Protocol(format!(
                    "{}: exit code {}, stderr: {}",
                    self.name,
                    output.status,
                    stderr.chars().take(200).collect::<String>()
                )))
            }
            Ok(Err(e)) => Err(ChatError::Unreachable(format!("{}: {}", self.name, e))),
            Err(_) => Err(ChatError::Timeout {
                ms: self.timeout.as_millis() as u64,
            }),
        }
    }
}
```

- [ ] **Step 6: Add `pub mod cli;` to backends/mod.rs**

```rust
pub mod cli;
pub mod embedding;
pub mod openai;
pub mod summarizer;
```

- [ ] **Step 7: Run all backend tests**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel backends::cli -- --nocapture
```

Expected: name test PASS, chat test PASS (or SKIP if gemini not installed)

- [ ] **Step 8: Commit**

```bash
git add cynic-kernel/src/backends/cli.rs cynic-kernel/src/backends/mod.rs
git commit -m "feat(backends): add CliBackend — ChatPort via subprocess for gemini-cli"
```

---

### Task 2: Config — backend_type field

**Files:**
- Modify: `cynic-kernel/src/infra/config.rs:96-122` (BackendEntry)
- Modify: `cynic-kernel/src/infra/config.rs:152-218` (load_backends)
- Modify: `cynic-kernel/src/infra/config.rs:260-280` (load_system_contract)

- [ ] **Step 1: Write the config parse test**

Add to `config.rs` tests:

```rust
#[test]
fn parse_cli_backend_type() {
    let toml_content = r#"
[backend.gemini-flash]
backend_type = "cli"
base_url = "gemini"
model = "gemini-2.5-flash"
auth_style = "none"
timeout_secs = 60
"#;
    let dir = std::env::temp_dir().join("cynic_config_cli_type_test");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("backends.toml");
    std::fs::write(&path, toml_content).unwrap();

    let configs = load_backends(&path);
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].backend_type, BackendType::Cli);
    assert_eq!(configs[0].name, "gemini-flash");
    // CLI backends: base_url holds the binary path
    assert_eq!(configs[0].base_url, "gemini");

    std::fs::remove_file(&path).ok();
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel config::tests::parse_cli_backend_type -- --exact
```

Expected: FAIL — `BackendType` not defined

- [ ] **Step 3: Add BackendType to BackendConfig**

In `config.rs`, add the enum and field:

```rust
/// Backend adapter type — determines which struct implements ChatPort.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BackendType {
    /// HTTP-based OpenAI-compatible API (default for all existing backends).
    #[default]
    OpenAi,
    /// CLI subprocess (e.g. gemini-cli). base_url holds the binary path.
    Cli,
}
```

Add to `BackendConfig` struct (after `name` field):
```rust
pub backend_type: BackendType,
```

Add to `BackendEntry` (TOML deserialization):
```rust
backend_type: Option<String>,
```

In `load_backends()`, inside the `filter_map` closure, parse it:
```rust
let backend_type = match entry.backend_type.as_deref() {
    Some("cli") => BackendType::Cli,
    Some("openai") | None => BackendType::OpenAi,
    Some(other) => {
        tracing::warn!(backend_type = %other, backend = %name, "unknown backend_type, defaulting to openai");
        BackendType::OpenAi
    }
};
```

Add `backend_type` to the `BackendConfig` construction in `Some(BackendConfig { ... })`.

For CLI backends, skip health_url derivation (they have no HTTP endpoint):
```rust
let health_url = if backend_type == BackendType::Cli {
    None
} else {
    entry.health_url
        .or_else(|| if is_sovereign { Some(derive_health_url(&entry.base_url)) } else { None })
};
```

- [ ] **Step 4: Fix all existing BackendConfig constructions**

Add `backend_type: BackendType::OpenAi` to:
- `load_backends_from_env()` (~line 290)
- All test constructors in `openai.rs` tests (4 instances)
- `register_dog_handler` in `api/rest/health.rs` (~line 340)

- [ ] **Step 5: Run test to verify it passes**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel config::tests::parse_cli_backend_type -- --exact
```

Expected: PASS

- [ ] **Step 6: Run full test suite**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel
```

Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/infra/config.rs cynic-kernel/src/backends/openai.rs cynic-kernel/src/api/rest/health.rs
git commit -m "feat(config): add backend_type field — cli vs openai adapter selection"
```

---

### Task 3: Wire CliBackend into boot (main.rs Ring 2)

**Files:**
- Modify: `cynic-kernel/src/main.rs:225-301`

- [ ] **Step 1: Add the branch in Ring 2 Dog construction**

In `main.rs`, replace the single `OpenAiCompatBackend::new(cfg.clone())` block with a match on `backend_type`:

```rust
for cfg in backend_configs {
    let backend: Arc<dyn ChatPort> = match cfg.backend_type {
        infra::config::BackendType::Cli => {
            // CLI backend: base_url is the binary path, no HTTP client needed
            Arc::new(backends::cli::CliBackend::new(
                &cfg.name,
                &cfg.base_url,
                cfg.timeout_secs,
            ))
        }
        infra::config::BackendType::OpenAi => {
            match backends::openai::OpenAiCompatBackend::new(cfg.clone()) {
                Ok(b) => Arc::new(b),
                Err(e) => {
                    klog!(
                        "[Ring 2] InferenceDog '{}' SKIPPED — HTTP client init failed: {}",
                        cfg.name, e
                    );
                    continue;
                }
            }
        }
    };
    let health = BackendPort::health(backend.as_ref()).await;
    // ... rest of Dog construction stays the same
```

**Key:** `InferenceDog::new()` takes `Arc<dyn ChatPort>` (it already does — the existing `Arc<OpenAiCompatBackend>` coerces to this). The rest of the Dog construction loop (organ registration, cost_rates, health_urls, fleet_meta, remediation) stays unchanged.

For CLI backends, skip fleet_meta (no base_url to probe):
```rust
if cfg.backend_type != infra::config::BackendType::Cli {
    fleet_meta.insert(cfg.name.clone(), (cfg.base_url.clone(), cfg.context_size, cfg.model.clone(), cfg.api_key.clone()));
}
```

- [ ] **Step 2: Build and verify**

```bash
export RUST_MIN_STACK=16777216
cargo build -p cynic-kernel --tests
```

Expected: compiles with 0 errors

- [ ] **Step 3: Run full test suite**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel
```

Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/main.rs
git commit -m "feat(boot): wire CliBackend into Ring 2 Dog construction"
```

---

### Task 4: backends.toml — add Gemini CLI entry

**Files:**
- Modify: `~/.config/cynic/backends.toml`
- Modify: `backends.toml.example`

- [ ] **Step 1: Add Gemini CLI entry to backends.toml**

Append to `~/.config/cynic/backends.toml`:

```toml
# ── Dog 5: Gemini CLI — Google subscription, no API key needed ──
# Uses gemini-cli binary (OAuth-based, subscription auth).
# Different model family (Google) = architectural diversity.
[backend.gemini-2-5-flash]
backend_type = "cli"
base_url = "gemini"
model = "gemini-2.5-flash"
auth_style = "none"
context_size = 1000000
timeout_secs = 60
max_tokens = 1024
temperature = 0.3
cost_input_per_mtok = 0.0
cost_output_per_mtok = 0.0
```

- [ ] **Step 2: Add CLI example to backends.toml.example**

Add a commented CLI example section.

- [ ] **Step 3: Commit**

```bash
git add backends.toml.example
git commit -m "docs(config): add CLI backend_type example for gemini-cli"
```

---

### Task 5: Dev domain prompt

**Files:**
- Create: `cynic-kernel/domains/dev.md`

- [ ] **Step 1: Create dev.md**

Following the chess.md pattern (H1 title, then H2 per axiom with HIGH/MEDIUM/LOW scoring guidance):

```markdown
# Dev Domain — Axiom Evaluation Criteria

Evaluate the CODE CHANGE, COMMIT, or ARCHITECTURAL DECISION described — not the quality of its textual description. A brilliant refactoring poorly described is still brilliant engineering. A regression described eloquently is still a regression.

## FIDELITY
Is this faithful to sound engineering principles? Does it follow established patterns, correct error handling, and proven design?
- HIGH (0.55-0.65): Follows SOLID principles, correct error handling, well-tested, addresses root cause. Example: extracting a shared function to eliminate duplication (DRY) with tests covering both callers.
- MEDIUM (0.25-0.45): Reasonable approach but minor issues — missing edge case, slightly over-engineered, acceptable but not ideal. Example: adding a retry loop without backoff.
- LOW (0.05-0.20): Violates fundamental principles — silent error swallowing, hardcoded secrets, untested critical path, copy-paste duplication. Example: `unwrap()` on user input in production code.

## PHI
Is this structurally harmonious? Are components well-proportioned? Is the abstraction level consistent?
- HIGH (0.55-0.65): Clean separation of concerns, consistent abstraction level, interfaces match responsibilities. Example: a port trait with one adapter, tested at the boundary.
- MEDIUM (0.25-0.45): Mostly clean but some leaky abstractions or mixed levels. Example: a domain function that imports an infra type but isolates it well.
- LOW (0.05-0.20): God objects, mixed concerns, abstraction mismatch. Example: a 500-line function mixing HTTP parsing, business logic, and database queries.

## VERIFY
Is this testable? Are claims verifiable? Can it be falsified?
- HIGH (0.55-0.65): Has tests, tests are meaningful (not tautological), covers edge cases, includes before/after measurement. Example: TDD — failing test written first, minimal implementation, then refactored.
- MEDIUM (0.25-0.45): Has some tests but gaps — happy path only, or tests exist but don't assert meaningful properties. Example: test that checks the function doesn't panic but not the output.
- LOW (0.05-0.20): Untested, untestable (tightly coupled), or claims "works" without evidence. Example: "I tested it manually" with no automated test.

## CULTURE
Does this honor the project's patterns and conventions? Does it respect the codebase's idioms?
- HIGH (0.55-0.65): Follows existing patterns exactly, naming consistent, file placement matches convention. Example: new backend follows the OpenAiCompatBackend pattern — same trait, same error handling, same test structure.
- MEDIUM (0.25-0.45): Mostly follows convention but introduces minor deviations. Example: using a different error type than the rest of the codebase.
- LOW (0.05-0.20): Ignores project conventions, introduces foreign patterns, breaks existing idioms. Example: adding a Python script to a Rust project for something the Rust toolchain handles.

## BURN
Is this efficient? Minimal waste? Does every line justify its existence?
- HIGH (0.55-0.65): Minimal code, no dead paths, no speculative abstractions, solves exactly the stated problem. Example: a 20-line function that replaces 200 lines of over-engineered code.
- MEDIUM (0.25-0.45): Reasonable but some waste — unnecessary abstractions, over-engineering for hypothetical futures. Example: adding a config option for something that has exactly one value.
- LOW (0.05-0.20): Bloated, dead code, premature abstractions, solves problems that don't exist. Example: a factory-builder-strategy pattern for a single implementation.

## SOVEREIGNTY
Does this preserve agency and independence? Does it avoid vendor lock-in? Does it keep control local?
- HIGH (0.55-0.65): Sovereign infrastructure, no new external dependencies, data stays local, can be modified freely. Example: implementing a feature using only the project's existing dependencies.
- MEDIUM (0.25-0.45): Introduces a dependency but behind a port trait, or uses a well-maintained open-source library. Example: adding a crate with an abstraction layer.
- LOW (0.05-0.20): Hard vendor lock-in, proprietary dependencies, data leaves the system, cannot be replaced. Example: calling a cloud API directly from domain code with no abstraction.
```

- [ ] **Step 2: Verify domain prompt loads**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel config::tests -- --nocapture 2>&1 | grep -i domain
```

The existing `load_domain_prompts` will pick it up from `cynic-kernel/domains/dev.md`.

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/domains/dev.md
git commit -m "feat(domains): add dev domain prompt — axiom criteria for code evaluation"
```

---

### Task 6: Nightshift loop — spawn_nightshift_loop

**Files:**
- Create: `cynic-kernel/src/infra/tasks/nightshift.rs`
- Modify: `cynic-kernel/src/infra/tasks.rs`
- Modify: `cynic-kernel/src/domain/constants.rs`
- Modify: `cynic-kernel/src/infra/task_health.rs`

- [ ] **Step 1: Add constants**

In `constants.rs`, add:

```rust
/// Nightshift — autonomous dev judgment loop.
pub const NIGHTSHIFT_INTERVAL: Duration = Duration::from_secs(4 * 3600); // 4 hours
/// Per-commit judgment timeout (includes all Dogs).
pub const NIGHTSHIFT_COMMIT_TIMEOUT: Duration = Duration::from_secs(300); // 5 min
/// Git lookback window for nightshift commit discovery.
pub const NIGHTSHIFT_GIT_LOOKBACK: &str = "24h";
```

- [ ] **Step 2: Add TaskHealth nightshift field**

In `task_health.rs`, add the `NIGHTSHIFT` contract constant (after `CRYSTAL_CHALLENGE`):

```rust
const NIGHTSHIFT: TaskContract = TaskContract {
    name: "nightshift",
    expected_interval: 4 * 3600 + 600, // 4h + 10min grace
    criticality: TaskCriticality::Housekeeping,
    consumer: "dev crystal formation",
    failure_effect: "autonomous dev judgment stops, no new dev crystals form overnight",
};
```

Add `nightshift: AtomicU64` field to the `TaskHealth` struct, `touch_nightshift()` method, and include it in `snapshot()` and `new()`. **Critical:** also add a `"nightshift" => NIGHTSHIFT` arm to the `task_contract()` match — without it, the binary will panic at runtime when `snapshot()` calls `task_contract("nightshift")`.

- [ ] **Step 3: Write the git_commits_since function test**

In `nightshift.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_git_log_output() {
        let raw = "abc1234 feat: add new feature\ndef5678 fix: handle edge case\n";
        let commits = parse_git_log(raw);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc1234");
        assert_eq!(commits[0].message, "feat: add new feature");
        assert_eq!(commits[1].hash, "def5678");
    }

    #[test]
    fn parse_git_log_empty() {
        let commits = parse_git_log("");
        assert!(commits.is_empty());
    }
}
```

- [ ] **Step 4: Implement nightshift.rs**

```rust
//! Nightshift — autonomous dev judgment loop.
//! Every 4h: git log --since=24h → judge each commit in domain="dev" → observe patterns.
//! Sovereign: runs inside the kernel process, no bash scripts.

use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::domain::constants;
use crate::domain::dog::Stimulus;
use crate::domain::metrics::Metrics;
use crate::domain::storage::StoragePort;
use crate::infra::task_health::TaskHealth;

/// A parsed git commit.
#[derive(Debug, Clone)]
struct GitCommit {
    hash: String,
    message: String,
}

/// Parse `git log --format="%h %s"` output into structured commits.
fn parse_git_log(raw: &str) -> Vec<GitCommit> {
    raw.lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let (hash, message) = line.split_once(' ')?;
            Some(GitCommit {
                hash: hash.to_string(),
                message: message.to_string(),
            })
        })
        .collect()
}

/// Fetch recent commits via `git log --since=<lookback> --format="%h %s"`.
/// Runs git as a subprocess — sovereign (no network, no external service).
async fn git_commits_since(lookback: &str, repo_path: &str) -> Result<Vec<GitCommit>, String> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::process::Command::new("git")
            .args([
                "-C", repo_path,
                "log",
                &format!("--since={lookback}"),
                "--format=%h %s",
            ])
            .output(),
    )
    .await
    .map_err(|_| "git log timed out (10s)".to_string())?
    .map_err(|e| format!("git log failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log exit {}: {}", output.status, stderr));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    Ok(parse_git_log(&raw))
}

/// Judge one commit: content = commit message, domain = "dev".
async fn judge_commit(
    commit: &GitCommit,
    judge: &Arc<crate::judge::Judge>,
    storage: &Arc<dyn StoragePort>,
) -> Result<(), String> {
    let stimulus = Stimulus {
        content: format!("{}: {}", commit.hash, commit.message),
        context: None,
        domain: Some("dev".to_string()),
        request_id: None,
    };

    let metrics = Metrics::new();
    let verdict = judge
        .evaluate(&stimulus, None, &metrics)
        .await
        .map_err(|e| format!("judge failed for {}: {e}", commit.hash))?;

    // Observe the verdict as a dev-domain pattern.
    // observe_crystal uses UPSERT — creates crystal if absent, updates if exists.
    // Crystal ID is deterministic from commit hash — same commit re-judged merges observations.
    let timestamp = chrono::Utc::now().to_rfc3339();
    storage
        .observe_crystal(
            &format!("nightshift-{}", commit.hash),
            &stimulus.content,
            "dev",
            verdict.q_score.total,
            &timestamp,
            verdict.voter_count,
            &verdict.id,
            &format!("{:?}", verdict.kind),
        )
        .await
        .map_err(|e| format!("observe failed for {}: {e}", commit.hash))?;

    tracing::info!(
        commit = %commit.hash,
        q_score = verdict.q_score.total,
        verdict = ?verdict.kind,
        dogs_used = verdict.voter_count,
        "nightshift judged commit"
    );

    Ok(())
}

pub fn spawn_nightshift_loop(
    judge: Arc<crate::judge::Judge>,
    storage: Arc<dyn StoragePort>,
    task_health: Arc<TaskHealth>,
    shutdown: CancellationToken,
    repo_path: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Wait 60s for system stabilization before first nightshift cycle
        tokio::select! {
            _ = shutdown.cancelled() => return,
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
        }

        let mut interval = tokio::time::interval(constants::NIGHTSHIFT_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        interval.tick().await; // skip first tick — first cycle runs after NIGHTSHIFT_INTERVAL, not immediately

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    klog!("[SHUTDOWN] Nightshift loop stopped");
                    break;
                }
                _ = interval.tick() => {
                    klog!("[Nightshift] Cycle starting — scanning commits since {}", constants::NIGHTSHIFT_GIT_LOOKBACK);

                    let commits = match git_commits_since(
                        constants::NIGHTSHIFT_GIT_LOOKBACK,
                        &repo_path,
                    ).await {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::warn!(error = %e, "nightshift: git log failed");
                            task_health.touch_nightshift();
                            continue;
                        }
                    };

                    if commits.is_empty() {
                        klog!("[Nightshift] No commits in last {} — cycle idle", constants::NIGHTSHIFT_GIT_LOOKBACK);
                        task_health.touch_nightshift();
                        continue;
                    }

                    klog!("[Nightshift] {} commit(s) to judge", commits.len());
                    let mut judged = 0;
                    let mut failed = 0;

                    for commit in &commits {
                        match tokio::time::timeout(
                            constants::NIGHTSHIFT_COMMIT_TIMEOUT,
                            judge_commit(commit, &judge, &storage),
                        ).await {
                            Ok(Ok(())) => judged += 1,
                            Ok(Err(e)) => {
                                tracing::warn!(error = %e, commit = %commit.hash, "nightshift: commit judgment failed");
                                failed += 1;
                            }
                            Err(_) => {
                                tracing::warn!(commit = %commit.hash, "nightshift: commit judgment timed out (5min)");
                                failed += 1;
                            }
                        }
                    }

                    klog!(
                        "[Nightshift] Cycle complete: {}/{} commits judged ({} failed)",
                        judged, commits.len(), failed
                    );
                    task_health.touch_nightshift();
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_git_log_output() {
        let raw = "abc1234 feat: add new feature\ndef5678 fix: handle edge case\n";
        let commits = parse_git_log(raw);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc1234");
        assert_eq!(commits[0].message, "feat: add new feature");
        assert_eq!(commits[1].hash, "def5678");
    }

    #[test]
    fn parse_git_log_empty() {
        let commits = parse_git_log("");
        assert!(commits.is_empty());
    }

    #[test]
    fn parse_git_log_single_word_message() {
        let raw = "a1b2c3d refactor\n";
        let commits = parse_git_log(raw);
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "refactor");
    }

    #[tokio::test]
    async fn nightshift_respects_shutdown() {
        let judge = Arc::new(crate::judge::Judge::new(vec![], vec![]));
        let storage: Arc<dyn StoragePort> = Arc::new(crate::domain::storage::NullStorage);
        let task_health = Arc::new(TaskHealth::new());
        let shutdown = CancellationToken::new();

        let handle = spawn_nightshift_loop(
            judge, storage, task_health, shutdown.clone(), "/tmp".to_string(),
        );
        shutdown.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(3), handle)
            .await
            .expect("nightshift should stop within 3s")
            .expect("task should not panic");
    }
}
```

- [ ] **Step 5: Wire nightshift into tasks.rs**

In `tasks.rs`, add the module declaration (after `mod runtime_loops;`):
```rust
mod nightshift;
```

And add a **separate** re-export line (NOT inside the existing `pub use runtime_loops::{...}` — that would be a syntax error):
```rust
pub use nightshift::spawn_nightshift_loop;
```

- [ ] **Step 6: Run tests**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel nightshift -- --nocapture
```

Expected: All nightshift tests pass

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/infra/tasks/nightshift.rs cynic-kernel/src/infra/tasks.rs cynic-kernel/src/domain/constants.rs cynic-kernel/src/infra/task_health.rs
git commit -m "feat(nightshift): spawn_nightshift_loop — 4h autonomous dev judgment cycle"
```

---

### Task 7: Spawn nightshift in main.rs

**Files:**
- Modify: `cynic-kernel/src/main.rs` (Ring 3 area where other loops are spawned)

- [ ] **Step 1: Find where loops are spawned and add nightshift**

After the other `spawn_*` calls in main.rs (look for `spawn_crystal_challenge_loop`), add:

```rust
// ─── Nightshift: autonomous dev judgment (every 4h) ───────
let _nightshift_handle = infra::tasks::spawn_nightshift_loop(
    judge.clone(),       // Arc<Judge> — same as crystal_challenge
    storage_port.clone(),
    task_health.clone(),
    shutdown.clone(),
    project_root.display().to_string(),
);
klog!("[Ring 3] Nightshift loop started (every 4h, git lookback 24h)");
```

- [ ] **Step 2: Build and verify**

```bash
export RUST_MIN_STACK=16777216
cargo build -p cynic-kernel
```

Expected: compiles

- [ ] **Step 3: Run full test suite**

```bash
export RUST_MIN_STACK=16777216
cargo test -p cynic-kernel
```

Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/main.rs
git commit -m "feat(boot): spawn nightshift loop in Ring 3"
```

---

### Task 8: Calibration corpus

**Files:**
- Create: `docs/domains/calibration.md` (NOT in `cynic-kernel/domains/` — that directory is auto-loaded as domain prompts by `load_domain_prompts`)

- [ ] **Step 1: Create the calibration corpus**

This file serves as a reference document for calibration runs. The nightshift loop or manual `/judge` calls will use these stimuli to measure Dog discrimination. It lives in `docs/` to avoid being loaded as a domain prompt.

```markdown
# Calibration Corpus — Dev Domain

Reference stimuli for measuring Dog discrimination on dev-domain content.
Run each through `/judge` with `domain="dev"` and compare Q-scores.
A good Dog should score GOOD stimuli > 0.382 (WAG+) and BAD stimuli < 0.382 (GROWL-).

## GOOD (expected WAG or HOWL)

### G1: Clean error handling
feat(api): replace unwrap() with proper error propagation using ? operator across all request handlers. Each error now carries context via anyhow::Context. Tests verify error messages reach the caller.

### G2: DRY extraction
refactor(pipeline): extract compute_qscore into shared module — was duplicated in rest.rs and mcp.rs. One function, two callers, three tests. Zero behavior change.

### G3: Port trait discipline
feat(storage): add CachePort trait with get/set/invalidate. Redis adapter implements it. Domain code imports only the trait. Integration test round-trips a value.

### G4: TDD cycle
feat(auth): add JWT validation middleware. Wrote failing test first (expired token returns 401), then minimal implementation, then edge cases (malformed, missing, valid). 4 tests, 30 lines of implementation.

### G5: Minimal sovereign fix
fix(health): circuit breaker was stuck open after recovery — reset_at wasn't cleared on successful probe. One-line fix, regression test added, before/after metrics documented.

### G6: Clean separation of concerns
refactor(judge): split 400-line judge.rs into judge.rs (orchestration), scoring.rs (phi-bounded math), prompt.rs (template construction). Each file < 150 lines, one responsibility, existing tests still pass.

### G7: Proper resource cleanup
feat(tasks): background loops now respect CancellationToken for graceful shutdown. Each loop uses tokio::select! on shutdown + interval. Tests verify sub-second shutdown on cancel.

### G8: Empirical validation
perf(pipeline): reduced /judge p95 latency from 2.3s to 0.8s by parallelizing Dog evaluation with FuturesUnordered instead of sequential awaits. Before/after flamegraph attached.

### G9: Configuration discipline
refactor(config): move all magic numbers to constants.rs with semantic names. CRYSTAL_CHALLENGE_INTERVAL, DOG_TTL_CHECK, etc. Single source of truth, grep-friendly.

### G10: Defensive coding
fix(crystal): observe_crystal now validates confidence is in [0.0, 1.0] before storing. Previously accepted any f64 — negative scores corrupted state machine transitions. Regression test added.

## BAD (expected GROWL or BARK)

### B1: Silent error swallowing
fix(api): added .ok() to all database calls to "prevent crashes". No logging, no retry, no fallback. Errors are now invisible. Ship it.

### B2: God function
feat(pipeline): added 600-line process_everything() that parses HTTP, validates auth, queries DB, scores with Dogs, stores verdict, sends Slack notification, and updates metrics. No tests.

### B3: Hardcoded secrets
feat(auth): added API key validation. Key is hardcoded as const API_KEY: &str = "sk-prod-abc123" in the source file. Works great in production.

### B4: Copy-paste duplication
feat(api): added /v2/judge endpoint. Copied entire judge_handler from v1, changed one field name. Both versions now need to be maintained independently. 200 lines duplicated.

### B5: Untested critical path
feat(billing): added payment processing. No tests because "it's too hard to mock Stripe". Manual testing confirmed it works on staging. Deployed to production.

### B6: Premature abstraction
refactor(utils): created AbstractStrategyFactoryBuilder<T, U, V> with 4 trait bounds for a function that has exactly one implementation and is called once. 150 lines of generics, 3 lines of logic.

### B7: Vendor lock-in
feat(storage): replaced SQLite with Firebase. All queries use Firebase-specific syntax directly in domain code. No abstraction layer. "We'll always use Firebase."

### B8: Hope-driven engineering
fix(timeout): increased timeout from 30s to 300s because "sometimes it's slow." No investigation into why it's slow. No metrics. No root cause analysis.

### B9: Dead architecture
feat(plugins): added plugin system with registry, lifecycle hooks, dependency injection, and hot-reload. Zero plugins exist. Zero planned. "We might need it someday."

### B10: Breaking change without migration
refactor(db): renamed all database columns from camelCase to snake_case. No migration script. No backward compatibility. Existing data is now orphaned. YOLO.
```

- [ ] **Step 2: Commit**

```bash
mkdir -p docs/domains
git add docs/domains/calibration.md cynic-kernel/domains/dev.md
git commit -m "feat(domains): add dev domain prompt + calibration corpus (10 good + 10 bad)"
```

---

### Task 9: Integration verification — E2E test

This task verifies the whole chain works together. It requires a running kernel.

- [ ] **Step 1: Build and deploy**

```bash
export RUST_MIN_STACK=16777216
cargo build -p cynic-kernel --release
# Deploy via /deploy skill or manual restart
```

- [ ] **Step 2: Verify Gemini Dog appears in /dogs**

```bash
source ~/.cynic-env
curl -s -H "Authorization: Bearer $CYNIC_API_KEY" "${CYNIC_REST_ADDR}/dogs"
```

Expected: `["deterministic-dog","qwen-7b-hf","qwen35-9b-gpu","gemma-4b-core","gemini-2-5-flash"]`

- [ ] **Step 3: Judge a stimulus and verify Gemini participates**

```bash
source ~/.cynic-env
curl -s -X POST -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  "${CYNIC_REST_ADDR}/judge" \
  -d '{"content": "refactor: extract compute_qscore into shared module", "domain": "dev"}'
```

Verify response contains `gemini-2-5-flash` in `dogs_used` array.

- [ ] **Step 4: Verify nightshift task appears in /health**

```bash
source ~/.cynic-env
curl -s "${CYNIC_REST_ADDR}/health" | python3 -m json.tool | grep nightshift
```

Expected: nightshift task visible in health output.

- [ ] **Step 5: Run one calibration stimulus manually**

```bash
source ~/.cynic-env
# Good stimulus (expect WAG+)
curl -s -X POST -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  "${CYNIC_REST_ADDR}/judge" \
  -d '{"content": "feat(api): replace unwrap() with proper error propagation using ? operator across all request handlers", "domain": "dev"}'

# Bad stimulus (expect GROWL-)
curl -s -X POST -H "Authorization: Bearer $CYNIC_API_KEY" \
  -H "Content-Type: application/json" \
  "${CYNIC_REST_ADDR}/judge" \
  -d '{"content": "fix(api): added .ok() to all database calls to prevent crashes. No logging, no retry, no fallback.", "domain": "dev"}'
```

Compare Q-scores. Good should score higher than bad. Record as baseline.

---

### Task 10: Clippy + full validation

- [ ] **Step 1: Run clippy**

```bash
export RUST_MIN_STACK=16777216
cargo clippy -p cynic-kernel --all -- -D warnings
```

Expected: 0 warnings

- [ ] **Step 2: Run full make check**

```bash
make check
```

Expected: all gates pass

- [ ] **Step 3: Final commit if any fixes needed**

```bash
git add -A
git commit -m "chore: clippy fixes for gemini-dog + nightshift"
```
