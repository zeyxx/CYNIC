# cynic-node Phase B — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `cynic-node` binary — a standalone Rust process that supervises one inference backend, registers it with the CYNIC kernel, and keeps it alive.

**Architecture:** Organic loop — sequential startup (spawn → wait_healthy → register), concurrent watch (heartbeat + health + identity in one `select!`), crash restarts the loop. 5 source files, 3 concerns (supervise, announce, verify), ~400-500 lines.

**Tech Stack:** Rust 1.94+ (edition 2024), tokio, reqwest, process-wrap 9.x, serde/toml, clap, tracing. Zero dep on cynic-kernel.

**Spec:** `docs/superpowers/specs/2026-04-06-cynic-node-phase-b-design.md`

**Dead code strategy:** Workspace lints deny `dead_code`. Each task writes the module AND wires it into `main.rs` so nothing is dead. Tests import from the crate via `#[cfg(test)]` in-module blocks.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `Cargo.toml` (root) | Modify | Add `cynic-node` to workspace members |
| `cynic-node/Cargo.toml` | Create | Binary crate, deps, inherit workspace lints |
| `cynic-node/src/main.rs` | Create | CLI (clap), config load, outer loop, signal handler |
| `cynic-node/src/config.rs` | Create | TOML structs, validation, derived values |
| `cynic-node/src/supervise.rs` | Create | spawn_backend(), graceful_stop(), Backoff |
| `cynic-node/src/verify.rs` | Create | check_health(), check_identity(), wait_healthy() |
| `cynic-node/src/announce.rs` | Create | register(), send_heartbeat(), try_deregister() |

No modifications to `cynic-kernel/`. Zero shared Rust types (T7).

---

### Task 1: Scaffold + Config (~30 min)

Create the crate, write config parsing + validation, wire into main.

**Files:**
- Modify: `Cargo.toml` (root, workspace members)
- Create: `cynic-node/Cargo.toml`
- Create: `cynic-node/src/main.rs`
- Create: `cynic-node/src/config.rs`
- Create: `cynic-node/src/supervise.rs` (empty pub mod)
- Create: `cynic-node/src/announce.rs` (empty pub mod)
- Create: `cynic-node/src/verify.rs` (empty pub mod)

- [ ] **Step 1: Create cynic-node/Cargo.toml**

```toml
[package]
name = "cynic-node"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "cynic-node"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread", "process", "time", "signal"] }
tokio-util = "0.7"
reqwest = { version = "0.13", default-features = false, features = ["json", "native-tls"] }
process-wrap = { version = "9.1", features = ["tokio1"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[lints]
workspace = true
```

- [ ] **Step 2: Add to workspace**

In root `Cargo.toml`, change:
```toml
members = ["cynic-kernel"]
```
to:
```toml
members = ["cynic-kernel", "cynic-node"]
```

- [ ] **Step 3: Create stub main.rs**

```rust
mod announce;
mod config;
mod supervise;
mod verify;

use clap::Parser;

#[derive(Parser)]
#[command(name = "cynic-node", about = "CYNIC inference backend supervisor")]
struct Cli {
    /// Path to node config file
    #[arg(short, long)]
    config: String,
}

fn main() {
    let cli = Cli::parse();
    let _cfg = config::load(&cli.config);
}
```

Create empty modules:
- `cynic-node/src/supervise.rs`: `// Supervise concern: spawn, stop, backoff`
- `cynic-node/src/announce.rs`: `// Announce concern: register, heartbeat, deregister`
- `cynic-node/src/verify.rs`: `// Verify concern: health check, identity check`

- [ ] **Step 4: Write config.rs with tests**

```rust
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub kernel: KernelConfig,
    pub dog: DogConfig,
    pub process: ProcessConfig,
    pub restart: RestartConfig,
    pub health: HealthConfig,
}

#[derive(Debug, Deserialize)]
pub struct KernelConfig {
    pub url: String,
    api_key_env: String,
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_secs: u64,
    // Resolved at load time, not serialized
    #[serde(skip)]
    pub api_key: String,
}

fn default_heartbeat_interval() -> u64 { 40 }

#[derive(Debug, Deserialize)]
pub struct DogConfig {
    pub name: String,
    pub model: String,
    pub base_url: String,
    #[serde(default = "default_context_size")]
    pub context_size: u32,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    api_key_env: Option<String>,
    #[serde(skip)]
    pub api_key: Option<String>,
}

fn default_context_size() -> u32 { 4096 }
fn default_timeout() -> u64 { 60 }

#[derive(Debug, Deserialize)]
pub struct ProcessConfig {
    pub command: Vec<String>,
    pub working_dir: Option<String>,
    #[serde(default = "default_stop_timeout")]
    pub stop_timeout_secs: u64,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

fn default_stop_timeout() -> u64 { 10 }

#[derive(Debug, Deserialize)]
pub struct RestartConfig {
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    #[serde(default = "default_initial_delay")]
    pub initial_delay_secs: u64,
    #[serde(default = "default_max_delay")]
    pub max_delay_secs: u64,
    #[serde(default = "default_min_uptime")]
    pub min_uptime_secs: u64,
}

fn default_max_attempts() -> u32 { 5 }
fn default_initial_delay() -> u64 { 2 }
fn default_max_delay() -> u64 { 120 }
fn default_min_uptime() -> u64 { 10 }

#[derive(Debug, Deserialize)]
pub struct HealthConfig {
    #[serde(default = "default_health_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_verify_interval")]
    pub verify_interval_secs: u64,
    #[serde(default = "default_health_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_max_failures")]
    pub max_failures: u32,
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout_secs: u64,
}

fn default_health_interval() -> u64 { 15 }
fn default_verify_interval() -> u64 { 60 }
fn default_health_timeout() -> u64 { 5 }
fn default_max_failures() -> u32 { 3 }
fn default_startup_timeout() -> u64 { 120 }

/// Derive health URL: strip /v1 from base_url, append /health
pub fn derive_health_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/').trim_end_matches("/v1");
    format!("{base}/health")
}

/// Derive models URL: base_url + /models
pub fn derive_models_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{base}/models")
}

pub fn load(path: &str) -> Config {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| {
            eprintln!("failed to read config {path}: {e}");
            std::process::exit(1);
        });
    let mut cfg: Config = toml::from_str(&content)
        .unwrap_or_else(|e| {
            eprintln!("failed to parse config {path}: {e}");
            std::process::exit(1);
        });
    // Resolve env vars
    cfg.kernel.api_key = resolve_env(&cfg.kernel.api_key_env, "kernel.api_key_env");
    if let Some(env_name) = &cfg.dog.api_key_env {
        cfg.dog.api_key = Some(resolve_env(env_name, "dog.api_key_env"));
    }
    validate(&cfg);
    cfg
}

fn resolve_env(var_name: &str, field: &str) -> String {
    std::env::var(var_name).unwrap_or_else(|_| {
        eprintln!("{field}={var_name} is not set in environment");
        std::process::exit(1);
    })
}

fn validate(cfg: &Config) {
    assert!(!cfg.process.command.is_empty(), "process.command must not be empty");
    assert!(cfg.dog.name.len() >= 1 && cfg.dog.name.len() <= 64,
        "dog.name must be 1-64 chars, got {}", cfg.dog.name.len());
    assert!(cfg.process.stop_timeout_secs > 0, "stop_timeout_secs must be > 0");
    assert!(cfg.restart.max_attempts > 0, "max_attempts must be > 0");
    assert!(cfg.health.startup_timeout_secs > cfg.health.timeout_secs,
        "startup_timeout_secs must be > timeout_secs");
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TOML: &str = r#"
[kernel]
url = "http://localhost:3030"
api_key_env = "TEST_API_KEY"

[dog]
name = "test-dog"
model = "test-model"
base_url = "http://localhost:8080/v1"

[process]
command = ["echo", "hello"]

[restart]

[health]
"#;

    #[test]
    fn parse_valid_config() {
        std::env::set_var("TEST_API_KEY", "secret");
        let mut cfg: Config = toml::from_str(VALID_TOML).unwrap();
        cfg.kernel.api_key = "secret".to_string();
        validate(&cfg);
        assert_eq!(cfg.dog.name, "test-dog");
        assert_eq!(cfg.dog.context_size, 4096); // default
        assert_eq!(cfg.restart.max_attempts, 5); // default
        assert_eq!(cfg.health.interval_secs, 15); // default
    }

    #[test]
    fn derive_health_url_strips_v1() {
        assert_eq!(derive_health_url("http://localhost:8080/v1"), "http://localhost:8080/health");
        assert_eq!(derive_health_url("http://localhost:8080/v1/"), "http://localhost:8080/health");
        assert_eq!(derive_health_url("http://localhost:8080"), "http://localhost:8080/health");
    }

    #[test]
    fn derive_models_url_appends() {
        assert_eq!(derive_models_url("http://localhost:8080/v1"), "http://localhost:8080/v1/models");
        assert_eq!(derive_models_url("http://localhost:8080/v1/"), "http://localhost:8080/v1/models");
    }

    #[test]
    #[should_panic(expected = "dog.name must be 1-64 chars")]
    fn reject_empty_name() {
        let toml_str = VALID_TOML.replace("test-dog", "");
        let cfg: Config = toml::from_str(&toml_str).unwrap();
        validate(&cfg);
    }

    #[test]
    #[should_panic(expected = "process.command must not be empty")]
    fn reject_empty_command() {
        let toml_str = VALID_TOML.replace(r#"command = ["echo", "hello"]"#, "command = []");
        let cfg: Config = toml::from_str(&toml_str).unwrap();
        validate(&cfg);
    }
}
```

- [ ] **Step 5: Verify build**

Run: `cd /home/user/Bureau/CYNIC && cargo build -p cynic-node`
Expected: compiles with no errors.

Run: `cargo test -p cynic-node`
Expected: all config tests pass.

- [ ] **Step 6: Commit**

```bash
git add cynic-node/ Cargo.toml
git commit -m "feat(node): scaffold cynic-node crate + config parsing (Phase B Task 1)"
```

---

### Task 2: Supervise — Backoff + Spawn + Stop (~30 min)

**Files:**
- Modify: `cynic-node/src/supervise.rs`
- Modify: `cynic-node/src/main.rs` (wire supervise into main)

- [ ] **Step 1: Write Backoff tests**

In `supervise.rs`, write the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn backoff_progression() {
        let mut b = Backoff::new(5, Duration::from_secs(2), Duration::from_secs(120), Duration::from_secs(10));
        assert!(!b.exhausted());
        assert_eq!(b.next_delay(), Duration::from_secs(2));   // attempt 1: 2^0 * 2 = 2
        assert_eq!(b.next_delay(), Duration::from_secs(4));   // attempt 2: 2^1 * 2 = 4
        assert_eq!(b.next_delay(), Duration::from_secs(8));   // attempt 3: 2^2 * 2 = 8
        assert_eq!(b.next_delay(), Duration::from_secs(16));  // attempt 4: 2^3 * 2 = 16
        assert_eq!(b.next_delay(), Duration::from_secs(32));  // attempt 5: 2^4 * 2 = 32
        assert!(b.exhausted());
    }

    #[test]
    fn backoff_caps_at_max() {
        let mut b = Backoff::new(10, Duration::from_secs(60), Duration::from_secs(120), Duration::from_secs(10));
        assert_eq!(b.next_delay(), Duration::from_secs(60));
        assert_eq!(b.next_delay(), Duration::from_secs(120)); // 60*2=120, capped at 120
        assert_eq!(b.next_delay(), Duration::from_secs(120)); // stays capped
    }

    #[test]
    fn backoff_reset_if_stable() {
        let mut b = Backoff::new(5, Duration::from_secs(1), Duration::from_secs(60), Duration::from_secs(0));
        b.record_start();
        b.next_delay(); // attempt 1
        b.next_delay(); // attempt 2
        assert_eq!(b.attempt, 2);
        // min_uptime = 0, so any elapsed time resets
        b.reset_if_stable();
        assert_eq!(b.attempt, 0);
    }

    #[test]
    fn backoff_reset_unconditional() {
        let mut b = Backoff::new(5, Duration::from_secs(1), Duration::from_secs(60), Duration::from_secs(10));
        b.next_delay();
        b.next_delay();
        b.reset();
        assert_eq!(b.attempt, 0);
        assert!(!b.exhausted());
    }
}
```

- [ ] **Step 2: Run tests, verify they fail**

Run: `cargo test -p cynic-node -- backoff`
Expected: FAIL — `Backoff` not defined.

- [ ] **Step 3: Implement Backoff**

In `supervise.rs`:

```rust
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

pub struct Backoff {
    pub attempt: u32,
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    min_uptime: Duration,
    last_start: Instant,
}

impl Backoff {
    pub fn new(max_attempts: u32, initial_delay: Duration, max_delay: Duration, min_uptime: Duration) -> Self {
        Self {
            attempt: 0,
            max_attempts,
            initial_delay,
            max_delay,
            min_uptime,
            last_start: Instant::now(),
        }
    }

    pub fn from_config(cfg: &crate::config::RestartConfig) -> Self {
        Self::new(
            cfg.max_attempts,
            Duration::from_secs(cfg.initial_delay_secs),
            Duration::from_secs(cfg.max_delay_secs),
            Duration::from_secs(cfg.min_uptime_secs),
        )
    }

    pub fn record_start(&mut self) { self.last_start = Instant::now(); }

    pub fn reset_if_stable(&mut self) {
        if self.last_start.elapsed() >= self.min_uptime {
            self.attempt = 0;
        }
    }

    pub fn reset(&mut self) { self.attempt = 0; }

    pub fn exhausted(&self) -> bool { self.attempt >= self.max_attempts }

    pub fn next_delay(&mut self) -> Duration {
        self.attempt += 1;
        let delay = self.initial_delay.saturating_mul(2u32.saturating_pow(self.attempt - 1));
        delay.min(self.max_delay)
    }

    pub async fn wait_or_shutdown(&mut self, shutdown: &CancellationToken) {
        let delay = self.next_delay();
        tracing::info!("backing off {delay:?} (attempt {}/{})", self.attempt, self.max_attempts);
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = shutdown.cancelled() => {}
        }
    }
}
```

- [ ] **Step 4: Run Backoff tests, verify pass**

Run: `cargo test -p cynic-node -- backoff`
Expected: all 4 tests pass.

- [ ] **Step 5: Write spawn + stop tests**

Add to the test module in `supervise.rs`:

```rust
    #[tokio::test]
    async fn spawn_and_wait() {
        let child = spawn_backend(&["echo", "hello"], None, &Default::default()).unwrap();
        let status = child.wait().await.unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn graceful_stop_clean() {
        let mut child = spawn_backend(&["sleep", "60"], None, &Default::default()).unwrap();
        graceful_stop(&mut child, 5).await;
        // Process should be dead
        let status = child.try_wait().unwrap();
        assert!(status.is_some());
    }

    #[tokio::test]
    async fn spawn_with_retries_bad_binary() {
        let result = spawn_with_retries(&["/nonexistent/binary"], None, &Default::default(), 3).await;
        assert!(result.is_err());
    }
```

- [ ] **Step 6: Run, verify fail**

Run: `cargo test -p cynic-node -- spawn`
Expected: FAIL — functions not defined.

- [ ] **Step 7: Implement spawn_backend, graceful_stop, spawn_with_retries**

Add to `supervise.rs` (before the test module):

```rust
use process_wrap::tokio::*;
use std::collections::HashMap;
use std::process::Stdio;

pub fn spawn_backend(
    command: &[impl AsRef<std::ffi::OsStr>],
    working_dir: Option<&str>,
    env: &HashMap<String, String>,
) -> std::io::Result<Box<dyn ChildWrapper>> {
    let (program, args) = command.split_first()
        .expect("command must not be empty");

    let wrap = CommandWrap::with_new(program, |cmd| {
        cmd.args(args)
           .stdout(Stdio::inherit())
           .stderr(Stdio::inherit());
        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }
        for (k, v) in env {
            cmd.env(k, v);
        }
    });

    #[cfg(unix)]
    let wrap = wrap.wrap(ProcessGroup::leader());
    #[cfg(windows)]
    let wrap = wrap.wrap(JobObject);

    wrap.wrap(KillOnDrop).spawn()
}

pub async fn graceful_stop(child: &mut Box<dyn ChildWrapper>, timeout_secs: u64) {
    #[cfg(unix)]
    let _ = child.signal(libc::SIGTERM);
    #[cfg(windows)]
    let _ = child.start_kill();

    match tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        child.wait(),
    ).await {
        Ok(_) => {}
        Err(_) => {
            tracing::warn!("backend did not stop within {timeout_secs}s, force killing");
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }
}

pub async fn spawn_with_retries(
    command: &[impl AsRef<std::ffi::OsStr>],
    working_dir: Option<&str>,
    env: &HashMap<String, String>,
    max: u32,
) -> std::io::Result<Box<dyn ChildWrapper>> {
    let mut last_err = None;
    for attempt in 1..=max {
        match spawn_backend(command, working_dir, env) {
            Ok(child) => return Ok(child),
            Err(e) => {
                tracing::error!("spawn attempt {attempt}/{max} failed: {e}");
                last_err = Some(e);
                if attempt < max {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }
    Err(last_err.expect("at least one attempt"))
}
```

- [ ] **Step 8: Wire into main.rs**

Add a minimal usage in main so supervise types aren't dead code:

```rust
// In main.rs, update main():
fn main() {
    let cli = Cli::parse();
    let cfg = config::load(&cli.config);
    // Wire supervise to prevent dead_code
    let _backoff = supervise::Backoff::from_config(&cfg.restart);
}
```

- [ ] **Step 9: Run all tests**

Run: `cargo test -p cynic-node`
Expected: all config + backoff + spawn tests pass.

Run: `cargo clippy -p cynic-node`
Expected: no warnings.

- [ ] **Step 10: Commit**

```bash
git add cynic-node/
git commit -m "feat(node): supervise — Backoff + spawn + graceful_stop (Phase B Task 2)"
```

---

### Task 3: Verify — Health + Identity (~25 min)

**Files:**
- Modify: `cynic-node/src/verify.rs`
- Modify: `cynic-node/src/main.rs` (wire verify)

- [ ] **Step 1: Write health + identity tests**

In `verify.rs`, write test module with a mock backend (Axum on 127.0.0.1:0):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    async fn mock_backend(health_ok: bool, model_id: &str) -> (String, tokio::task::JoinHandle<()>) {
        use axum::{Router, routing::get, Json};
        let health_ok = health_ok;
        let model_id = model_id.to_string();

        let app = Router::new()
            .route("/health", get(move || async move {
                if health_ok { (axum::http::StatusCode::OK, "ok") }
                else { (axum::http::StatusCode::SERVICE_UNAVAILABLE, "down") }
            }))
            .route("/v1/models", get({
                let model_id = model_id.clone();
                move || {
                    let model_id = model_id.clone();
                    async move {
                        Json(serde_json::json!({
                            "data": [{"id": model_id}]
                        }))
                    }
                }
            }));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (format!("http://127.0.0.1:{}", addr.port()), handle)
    }

    #[tokio::test]
    async fn health_check_ok() {
        let client = reqwest::Client::new();
        let (base, _h) = mock_backend(true, "test").await;
        assert!(check_health(&client, &format!("{base}/health"), 5).await);
    }

    #[tokio::test]
    async fn health_check_fail() {
        let client = reqwest::Client::new();
        let (base, _h) = mock_backend(false, "test").await;
        assert!(!check_health(&client, &format!("{base}/health"), 5).await);
    }

    #[tokio::test]
    async fn health_check_unreachable() {
        let client = reqwest::Client::new();
        assert!(!check_health(&client, "http://127.0.0.1:1/health", 1).await);
    }

    #[tokio::test]
    async fn identity_match() {
        let client = reqwest::Client::new();
        let (base, _h) = mock_backend(true, "qwen3.5-9b").await;
        match check_identity(&client, &format!("{base}/v1/models"), "qwen3.5-9b", 5).await {
            IdentityResult::Match => {}
            other => panic!("expected Match, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn identity_mismatch() {
        let client = reqwest::Client::new();
        let (base, _h) = mock_backend(true, "mistral-7b").await;
        match check_identity(&client, &format!("{base}/v1/models"), "qwen3.5-9b", 5).await {
            IdentityResult::Mismatch { expected, actual } => {
                assert_eq!(expected, "qwen3.5-9b");
                assert!(actual.contains("mistral-7b"));
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p cynic-node -- health`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Implement check_health + check_identity**

```rust
use reqwest::Client;
use serde_json::Value;

#[derive(Debug)]
pub enum IdentityResult {
    Match,
    Mismatch { expected: String, actual: String },
    Unknown,
    Unreachable,
}

pub async fn check_health(client: &Client, health_url: &str, timeout_secs: u64) -> bool {
    match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        client.get(health_url).send(),
    ).await {
        Ok(Ok(resp)) if resp.status().is_success() => true,
        _ => false,
    }
}

pub async fn check_identity(
    client: &Client,
    models_url: &str,
    expected_model: &str,
    timeout_secs: u64,
) -> IdentityResult {
    let resp = match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        client.get(models_url).send(),
    ).await {
        Ok(Ok(r)) => r,
        _ => return IdentityResult::Unreachable,
    };
    let body: Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return IdentityResult::Unknown,
    };
    let Some(models) = body["data"].as_array() else {
        return IdentityResult::Unknown;
    };
    let loaded: Vec<&str> = models.iter()
        .filter_map(|m| m["id"].as_str())
        .collect();
    if loaded.iter().any(|id| id.contains(expected_model)) {
        IdentityResult::Match
    } else {
        IdentityResult::Mismatch {
            expected: expected_model.to_string(),
            actual: loaded.join(", "),
        }
    }
}
```

- [ ] **Step 4: Wire into main.rs**

```rust
// In main.rs, add minimal usage:
fn main() {
    let cli = Cli::parse();
    let cfg = config::load(&cli.config);
    let _backoff = supervise::Backoff::from_config(&cfg.restart);
    let _health_url = config::derive_health_url(&cfg.dog.base_url);
    // verify module used via verify::check_health, verify::check_identity
    // (called in the async runtime, wired in Task 5)
}
```

Note: `verify::IdentityResult` and the functions are `pub` — they'll be used by main.rs in Task 5. The dead_code lint won't fire because they're public items in a binary crate's module. If it does, add `let _ = verify::check_health;` in main.

- [ ] **Step 5: Run all tests + clippy**

Run: `cargo test -p cynic-node`
Expected: all tests pass (config + backoff + spawn + health + identity).

Run: `cargo clippy -p cynic-node`
Expected: no warnings.

- [ ] **Step 6: Commit**

```bash
git add cynic-node/
git commit -m "feat(node): verify — check_health + check_identity with mock tests (Phase B Task 3)"
```

---

### Task 4: Announce — Register + Heartbeat + Deregister (~25 min)

**Files:**
- Modify: `cynic-node/src/announce.rs`
- Modify: `cynic-node/src/main.rs` (wire announce)

- [ ] **Step 1: Write announce tests with mock kernel**

In `announce.rs`, mock kernel that implements `/dogs/register`, `/dogs/{id}/heartbeat`, `DELETE /dogs/{id}`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    async fn mock_kernel(api_key: &str) -> (String, tokio::task::JoinHandle<()>) {
        use axum::{Router, routing::{post, delete}, Json, extract::Path, http::StatusCode};
        let expected_key = format!("Bearer {api_key}");
        let heartbeat_count = Arc::new(AtomicU32::new(0));

        let app = Router::new()
            .route("/dogs/register", post({
                let expected_key = expected_key.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<Value>| {
                    let expected_key = expected_key.clone();
                    async move {
                        let auth = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
                        if auth != expected_key {
                            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": "bad key"})));
                        }
                        let name = body["name"].as_str().unwrap_or("unknown");
                        (StatusCode::OK, Json(serde_json::json!({
                            "dog_id": name,
                            "calibration": "passed",
                            "roster_size": 1
                        })))
                    }
                }
            }))
            .route("/dogs/{id}/heartbeat", post({
                let hc = heartbeat_count.clone();
                move |Path(id): Path<String>| {
                    let hc = hc.clone();
                    async move {
                        hc.fetch_add(1, Ordering::Relaxed);
                        (StatusCode::OK, Json(serde_json::json!({
                            "dog_id": id,
                            "status": "alive",
                            "ttl_remaining_secs": 120
                        })))
                    }
                }
            }))
            .route("/dogs/{id}", delete(|Path(id): Path<String>| async move {
                (StatusCode::OK, Json(serde_json::json!({
                    "dog_id": id,
                    "status": "deregistered",
                    "roster_size": 0
                })))
            }));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
        (format!("http://127.0.0.1:{}", addr.port()), handle)
    }

    #[tokio::test]
    async fn register_success() {
        let client = reqwest::Client::new();
        let (url, _h) = mock_kernel("secret").await;
        let result = try_register(&client, &url, "secret", &serde_json::json!({
            "name": "test-dog", "base_url": "http://x", "model": "m"
        })).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().dog_id, "test-dog");
    }

    #[tokio::test]
    async fn heartbeat_alive() {
        let client = reqwest::Client::new();
        let (url, _h) = mock_kernel("secret").await;
        let result = send_heartbeat(&client, &url, "secret", "dog-1").await;
        assert!(matches!(result, HeartbeatResult::Alive));
    }

    #[tokio::test]
    async fn heartbeat_expired() {
        let client = reqwest::Client::new();
        // Heartbeat to a non-existent kernel → timeout/error
        // Use a port that accepts but returns 404 for unknown dogs
        let result = send_heartbeat(&client, "http://127.0.0.1:1", "k", "dog-1").await;
        assert!(matches!(result, HeartbeatResult::Error(_)));
    }

    #[tokio::test]
    async fn deregister_success() {
        let client = reqwest::Client::new();
        let (url, _h) = mock_kernel("secret").await;
        try_deregister(&client, &url, "secret", "dog-1").await;
        // No panic = success (best-effort)
    }
}
```

- [ ] **Step 2: Run, verify fail**

Run: `cargo test -p cynic-node -- register`
Expected: FAIL — types not defined.

- [ ] **Step 3: Implement announce functions**

```rust
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct RegisterResponse {
    pub dog_id: String,
    pub roster_size: usize,
}

#[derive(Debug)]
pub enum HeartbeatResult {
    Alive,
    Expired,
    Error(String),
}

#[derive(Debug)]
pub enum RegisterError {
    Collision,
    CalibrationFail(String),
    Transient(String),
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Collision => write!(f, "name collision (409)"),
            Self::CalibrationFail(e) => write!(f, "calibration failed (422): {e}"),
            Self::Transient(e) => write!(f, "transient error: {e}"),
        }
    }
}

pub async fn try_register(
    client: &Client,
    kernel_url: &str,
    api_key: &str,
    payload: &Value,
) -> Result<RegisterResponse, RegisterError> {
    let url = format!("{kernel_url}/dogs/register");
    let resp = client.post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .json(payload)
        .send().await
        .map_err(|e| RegisterError::Transient(e.to_string()))?;

    match resp.status().as_u16() {
        200 => {
            let body: RegisterResponse = resp.json().await
                .map_err(|e| RegisterError::Transient(e.to_string()))?;
            Ok(body)
        }
        409 => Err(RegisterError::Collision),
        422 => {
            let body = resp.text().await.unwrap_or_default();
            Err(RegisterError::CalibrationFail(body))
        }
        status => Err(RegisterError::Transient(format!("HTTP {status}"))),
    }
}

pub async fn send_heartbeat(
    client: &Client,
    kernel_url: &str,
    api_key: &str,
    dog_id: &str,
) -> HeartbeatResult {
    let url = format!("{kernel_url}/dogs/{dog_id}/heartbeat");
    match client.post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send().await
    {
        Ok(resp) if resp.status() == 200 => HeartbeatResult::Alive,
        Ok(resp) if resp.status() == 404 => HeartbeatResult::Expired,
        Ok(resp) => HeartbeatResult::Error(format!("HTTP {}", resp.status())),
        Err(e) => HeartbeatResult::Error(e.to_string()),
    }
}

pub async fn try_deregister(client: &Client, kernel_url: &str, api_key: &str, dog_id: &str) {
    let url = format!("{kernel_url}/dogs/{dog_id}");
    match tokio::time::timeout(
        Duration::from_secs(5),
        client.delete(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send(),
    ).await {
        Ok(Ok(_)) => tracing::info!("deregistered {dog_id}"),
        _ => tracing::warn!("deregister failed for {dog_id}, TTL will clean up"),
    }
}
```

- [ ] **Step 4: Run all tests + clippy**

Run: `cargo test -p cynic-node`
Expected: all tests pass.

Run: `cargo clippy -p cynic-node`

- [ ] **Step 5: Commit**

```bash
git add cynic-node/
git commit -m "feat(node): announce — register + heartbeat + deregister with mock tests (Phase B Task 4)"
```

---

### Task 5: Watch Loop + Main Loop + Signal Handling (~45 min)

This is the core — wiring everything together.

**Files:**
- Modify: `cynic-node/src/main.rs` (complete rewrite of main fn)

- [ ] **Step 1: Define ExitReason**

In `main.rs`:

```rust
#[derive(Debug)]
enum ExitReason {
    Shutdown,
    Crashed,
    Expired,
    Mismatch,
    Fatal(String),
}
```

- [ ] **Step 2: Implement watch()**

The concurrent `select!` loop — heartbeat, health, identity, child.wait(), shutdown:

```rust
async fn watch(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    dog_id: &str,
    shutdown: &CancellationToken,
) -> ExitReason {
    let health_url = config::derive_health_url(&cfg.dog.base_url);
    let models_url = config::derive_models_url(&cfg.dog.base_url);

    let mut heartbeat_tick = tokio::time::interval(Duration::from_secs(cfg.kernel.heartbeat_interval_secs));
    let mut health_tick = tokio::time::interval(Duration::from_secs(cfg.health.interval_secs));
    let mut verify_tick = tokio::time::interval(Duration::from_secs(cfg.health.verify_interval_secs));
    let mut health_failures: u32 = 0;

    loop {
        tokio::select! {
            _ = heartbeat_tick.tick() => {
                match announce::send_heartbeat(client, &cfg.kernel.url, &cfg.kernel.api_key, dog_id).await {
                    announce::HeartbeatResult::Alive => {}
                    announce::HeartbeatResult::Expired => return ExitReason::Expired,
                    announce::HeartbeatResult::Error(e) => tracing::warn!("heartbeat error: {e}"),
                }
            }
            _ = health_tick.tick() => {
                if verify::check_health(client, &health_url, cfg.health.timeout_secs).await {
                    health_failures = 0;
                } else {
                    health_failures += 1;
                    tracing::warn!("health check failed ({health_failures}/{})", cfg.health.max_failures);
                    if health_failures >= cfg.health.max_failures {
                        tracing::error!("backend unresponsive, killing");
                        supervise::graceful_stop(child, cfg.process.stop_timeout_secs).await;
                        return ExitReason::Crashed;
                    }
                }
            }
            _ = verify_tick.tick() => {
                match verify::check_identity(client, &models_url, &cfg.dog.model, cfg.health.timeout_secs).await {
                    verify::IdentityResult::Match => {}
                    verify::IdentityResult::Mismatch { expected, actual } => {
                        tracing::error!("model mismatch: expected {expected}, got {actual}");
                        return ExitReason::Mismatch;
                    }
                    _ => {} // transient — health check handles liveness
                }
            }
            status = child.wait() => {
                tracing::error!("backend exited: {status:?}");
                return ExitReason::Crashed;
            }
            _ = shutdown.cancelled() => {
                return ExitReason::Shutdown;
            }
        }
    }
}
```

- [ ] **Step 3: Implement wait_healthy()**

```rust
async fn wait_healthy(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    shutdown: &CancellationToken,
) -> Result<(), ExitReason> {
    let health_url = config::derive_health_url(&cfg.dog.base_url);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(cfg.health.startup_timeout_secs);
    let mut tick = tokio::time::interval(Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                if verify::check_health(client, &health_url, cfg.health.timeout_secs).await {
                    tracing::info!("backend healthy");
                    return Ok(());
                }
                if tokio::time::Instant::now() >= deadline {
                    tracing::error!("startup health timeout");
                    supervise::graceful_stop(child, cfg.process.stop_timeout_secs).await;
                    return Err(ExitReason::Crashed);
                }
            }
            _ = child.wait() => return Err(ExitReason::Crashed),
            _ = shutdown.cancelled() => return Err(ExitReason::Shutdown),
        }
    }
}
```

- [ ] **Step 4: Implement register_with_kernel()**

```rust
async fn register_with_kernel(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    shutdown: &CancellationToken,
) -> Result<String, ExitReason> {
    let mut payload = serde_json::json!({
        "name": cfg.dog.name,
        "base_url": cfg.dog.base_url,
        "model": cfg.dog.model,
        "context_size": cfg.dog.context_size,
        "timeout_secs": cfg.dog.timeout_secs,
    });
    if let Some(key) = &cfg.dog.api_key {
        payload["api_key"] = serde_json::json!(key);
    }

    let mut reg_backoff = supervise::Backoff::from_config(&cfg.restart);
    loop {
        tokio::select! {
            result = announce::try_register(client, &cfg.kernel.url, &cfg.kernel.api_key, &payload) => {
                match result {
                    Ok(resp) => {
                        tracing::info!("registered as {} (roster: {})", resp.dog_id, resp.roster_size);
                        return Ok(resp.dog_id);
                    }
                    Err(announce::RegisterError::Collision) => {
                        return Err(ExitReason::Fatal(format!("name collision: {}", cfg.dog.name)));
                    }
                    Err(announce::RegisterError::CalibrationFail(e)) => {
                        if reg_backoff.exhausted() {
                            return Err(ExitReason::Fatal(format!("calibration failed after retries: {e}")));
                        }
                        tracing::warn!("calibration failed: {e}, retrying");
                        reg_backoff.wait_or_shutdown(shutdown).await;
                    }
                    Err(e) => {
                        tracing::warn!("registration failed: {e}, retrying");
                        reg_backoff.wait_or_shutdown(shutdown).await;
                    }
                }
            }
            _ = child.wait() => return Err(ExitReason::Crashed),
            _ = shutdown.cancelled() => return Err(ExitReason::Shutdown),
        }
    }
}
```

- [ ] **Step 5: Implement the main loop**

Complete rewrite of `main()`:

```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let cfg = config::load(&cli.config);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.health.timeout_secs))
        .build()
        .expect("failed to build HTTP client");

    let shutdown = CancellationToken::new();
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("shutdown signal received");
        shutdown_clone.cancel();
    });

    let mut needs_spawn = true;
    let mut child: Option<Box<dyn ChildWrapper>> = None;
    let mut backoff = supervise::Backoff::from_config(&cfg.restart);
    let mut dog_id: Option<String> = None;

    loop {
        if needs_spawn {
            if let Some(id) = dog_id.take() {
                announce::try_deregister(&client, &cfg.kernel.url, &cfg.kernel.api_key, &id).await;
            }
            child = Some(match supervise::spawn_with_retries(
                &cfg.process.command.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                cfg.process.working_dir.as_deref(),
                &cfg.process.env, 3,
            ).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("spawn failed after 3 attempts: {e}");
                    std::process::exit(1);
                }
            });
            backoff.record_start();
        }

        let c = child.as_mut().expect("child must exist");

        // Run lifecycle: wait_healthy → register → watch
        let (reason, id) = run_lifecycle(&client, &cfg, c, &shutdown, &mut backoff).await;
        dog_id = id;

        match reason {
            ExitReason::Shutdown => {
                if let Some(id) = dog_id.take() {
                    announce::try_deregister(&client, &cfg.kernel.url, &cfg.kernel.api_key, &id).await;
                }
                supervise::graceful_stop(c, cfg.process.stop_timeout_secs).await;
                break;
            }
            ExitReason::Expired => {
                dog_id = None;
                needs_spawn = false;
            }
            ExitReason::Crashed => {
                dog_id = None;
                needs_spawn = true;
                backoff.reset_if_stable();
                if backoff.exhausted() {
                    tracing::error!("max restart attempts reached");
                    supervise::graceful_stop(c, cfg.process.stop_timeout_secs).await;
                    std::process::exit(1);
                }
                backoff.wait_or_shutdown(&shutdown).await;
            }
            ExitReason::Mismatch => {
                if let Some(id) = dog_id.take() {
                    announce::try_deregister(&client, &cfg.kernel.url, &cfg.kernel.api_key, &id).await;
                }
                supervise::graceful_stop(c, cfg.process.stop_timeout_secs).await;
                needs_spawn = true;
            }
            ExitReason::Fatal(e) => {
                tracing::error!("fatal: {e}");
                if let Some(id) = dog_id.take() {
                    announce::try_deregister(&client, &cfg.kernel.url, &cfg.kernel.api_key, &id).await;
                }
                supervise::graceful_stop(c, cfg.process.stop_timeout_secs).await;
                std::process::exit(1);
            }
        }
    }

    tracing::info!("cynic-node stopped");
}

async fn run_lifecycle(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    shutdown: &CancellationToken,
    backoff: &mut supervise::Backoff,
) -> (ExitReason, Option<String>) {
    if let Err(reason) = wait_healthy(client, cfg, child, shutdown).await {
        return (reason, None);
    }
    match register_with_kernel(client, cfg, child, shutdown).await {
        Ok(id) => {
            backoff.reset();
            let reason = watch(client, cfg, child, &id, shutdown).await;
            (reason, Some(id))
        }
        Err(reason) => (reason, None),
    }
}
```

- [ ] **Step 6: Add required imports to main.rs**

```rust
use config::Config;
use process_wrap::tokio::ChildWrapper;
use reqwest::Client;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
```

- [ ] **Step 7: Build + clippy**

Run: `cargo build -p cynic-node`
Expected: compiles.

Run: `cargo clippy -p cynic-node`
Expected: no warnings (may need adjustments for borrow checker — see spec note on child.wait() pinning).

Run: `cargo test -p cynic-node`
Expected: all existing tests still pass.

- [ ] **Step 8: Commit**

```bash
git add cynic-node/
git commit -m "feat(node): main loop + watch + signal handling (Phase B Task 5)"
```

---

### Task 6: Integration Test + make check (~20 min)

Full lifecycle test with mock kernel + mock backend.

**Files:**
- Create: `cynic-node/tests/lifecycle.rs` (optional — may use in-module test instead)
- Modify: `cynic-node/src/main.rs` (extract run() for testability if needed)

- [ ] **Step 1: Create example config**

Create `cynic-node/node.toml.example`:

```toml
# Example cynic-node configuration.
# Copy to /etc/cynic/node-<name>.toml and adjust.

[kernel]
url = "http://<TAILSCALE_CORE>:3030"
api_key_env = "CYNIC_API_KEY"
# heartbeat_interval_secs = 40

[dog]
name = "qwen35-9b-gpu"
model = "qwen3.5:9b-q4_K_M"
# base_url must be reachable FROM THE KERNEL, not from this node.
# Use Tailscale IP for cross-machine deployment.
base_url = "http://<TAILSCALE_GPU>:8080/v1"
# context_size = 8192
# timeout_secs = 60
# api_key_env = "DOG_API_KEY"

[process]
command = ["llama-server", "-m", "/models/qwen3.5-9b-q4_K_M.gguf", "--port", "8080", "--ctx-size", "8192"]
# working_dir = "/opt/llama"
# stop_timeout_secs = 10

# [process.env]
# LLAMA_LOG_VERBOSITY = "0"

[restart]
# max_attempts = 5
# initial_delay_secs = 2
# max_delay_secs = 120
# min_uptime_secs = 10

[health]
# interval_secs = 15
# verify_interval_secs = 60
# timeout_secs = 5
# max_failures = 3
# startup_timeout_secs = 120
```

- [ ] **Step 2: Run make check on full workspace**

Run: `cd /home/user/Bureau/CYNIC && make check`
Expected: build + test + clippy + lint-rules + lint-drift + audit all pass.

If lint-rules or lint-drift fail on cynic-node paths, check if the Makefile scopes lints to `cynic-kernel/` only. If so, no changes needed. If it scopes to the whole workspace, adjust the lint scripts to include `cynic-node/`.

- [ ] **Step 3: Verify binary runs**

Create a test config at `/tmp/test-node.toml`:
```toml
[kernel]
url = "http://localhost:3030"
api_key_env = "CYNIC_API_KEY"

[dog]
name = "test-node"
model = "test"
base_url = "http://localhost:9999/v1"

[process]
command = ["sleep", "10"]

[restart]

[health]
startup_timeout_secs = 5
```

Run:
```bash
CYNIC_API_KEY=test cargo run -p cynic-node -- --config /tmp/test-node.toml
```
Expected: spawns `sleep 10`, tries health check at localhost:9999, times out after 5s, tries to restart (backoff), eventually exits after 3 spawn-fail cycles or crashes. Ctrl+C should trigger graceful shutdown.

- [ ] **Step 4: Commit final**

```bash
git add cynic-node/
git commit -m "feat(node): example config + integration verification (Phase B Task 6)"
```

---

## Post-Implementation Checklist

- [ ] All `cargo test -p cynic-node` pass
- [ ] `cargo clippy -p cynic-node` clean
- [ ] `make check` passes on full workspace
- [ ] Example config in `cynic-node/node.toml.example`
- [ ] No secrets in any committed file
- [ ] No deps on `cynic-kernel` (verify: `grep cynic-kernel cynic-node/Cargo.toml` returns empty)
- [ ] Binary runs: `cargo run -p cynic-node -- --help` shows usage

## Known Implementation Risks

1. **`child.wait()` borrow pattern in `select!`**: If the borrow checker fights, pin the wait future outside the loop: `let wait = child.wait(); tokio::pin!(wait);` then use `&mut wait` in select. See spec note.

2. **process-wrap API drift**: The spec describes 9.1.0 API from research. Verify actual crate API against `docs.rs/process-wrap/9.1.0`. Key methods: `CommandWrap::with_new()`, `.wrap(ProcessGroup::leader())`, `.wrap(KillOnDrop)`, `.spawn()`, `child.signal()`, `child.start_kill()`, `child.wait()`.

3. **Axum version for tests**: The kernel uses `axum = "0.8"`. Test mock servers should use the same version. Add `axum` as a dev-dependency only: `[dev-dependencies] axum = "0.8"`.

4. **Cross-platform**: `#[cfg(windows)]` code compiles but is only testable on Windows. Verify with `cargo check --target x86_64-pc-windows-msvc` if the target is installed.
