# cynic-node Phase B — Design Spec

**Date:** 2026-04-06
**Status:** Draft
**Crystallized truths:** T1–T10 from `memory/project_cynic_node_design.md`
**Research inputs:** openfang config, llmfit detection, process-wrap 9.x API, supervisord/s6/systemd/PM2/foreman comparison

## Problem

CYNIC's inference backends (llama-server instances) are managed manually — systemd units, Windows schtasks. This creates:

- **Silent model mismatch**: GPU schtask `CynicMistral` serves Mistral but config says Gemma 4. Nobody detects this.
- **No self-healing**: If a backend crashes at 3am, it stays dead until a human restarts it.
- **Registration is manual**: Dogs are either hardcoded in `backends.toml` (permanent) or manually registered via API.

`cynic-node` is a standalone Rust binary that supervises one inference backend, registers it with the CYNIC kernel, and keeps it alive.

## Scope

Phase B delivers the `cynic-node` binary. Phase A (kernel TTL + heartbeat + deregister endpoints) is already deployed.

**In scope:** supervise, announce, verify — one node per backend, cross-platform (Linux + Windows), TOML config.
**Out of scope:** Phase C push mode (WebSocket), multi-dog per node, log rotation (delegate to systemd/journal).

## Architecture

### The organic loop

The program is a loop. Startup is sequential. Watch is concurrent. Crash restarts the loop.

```
BOOT ──▶ SPAWN ──▶ WAIT_HEALTHY ──▶ REGISTER ──▶ WATCH ──┐
  ▲                                                        │
  │         Crashed/Mismatch (backoff)                     │
  └────────────────────────────────────────────────────────┘
                         │
                    Expired (no restart, re-register)
                         │
                    Shutdown (deregister, stop, exit)
                         │
                    Fatal (stop, exit(1))
```

Every phase races against `child.wait()` (backend crash) and `shutdown` (SIGTERM/SIGINT). No phase can hang ignoring these signals.

### Core loop (pseudocode)

```rust
let shutdown = CancellationToken::new();
tokio::spawn(signal_handler(shutdown.clone()));

let mut needs_spawn = true;
let mut child: Option<Box<dyn ChildWrapper>> = None;
let mut backoff = Backoff::new(&config.restart);
let mut dog_id: Option<String> = None;

loop {
    if needs_spawn {
        // Best-effort deregister stale registration
        if let Some(id) = dog_id.take() {
            try_deregister(&config, &id).await;
        }

        child = Some(match spawn_with_retries(&config, 3) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("spawn failed after 3 attempts: {e}");
                std::process::exit(1);
            }
        });
    }

    let c = child.as_mut().unwrap();
    let result = run_lifecycle(&config, c, &shutdown, &mut backoff).await;

    match result {
        Shutdown => {
            if let Some(id) = dog_id.take() {
                try_deregister(&config, &id).await; // best-effort
            }
            graceful_stop(c, config.process.stop_timeout_secs).await;
            break;
        }
        Expired => {
            dog_id = None;
            needs_spawn = false;
            // backend still alive, re-register on next iteration
        }
        Crashed => {
            dog_id = None;
            needs_spawn = true;
            if backoff.exhausted() {
                tracing::error!("max restart attempts reached");
                std::process::exit(1);
            }
            backoff.wait_or_shutdown(&shutdown).await;
        }
        Mismatch => {
            needs_spawn = true;
            // restart backend to reload correct model
        }
        Fatal(e) => {
            tracing::error!("fatal: {e}");
            if let Some(c) = child.as_mut() {
                graceful_stop(c, config.process.stop_timeout_secs).await;
            }
            std::process::exit(1);
        }
    }
}

async fn run_lifecycle(cfg, child, shutdown, backoff) -> ExitReason {
    wait_healthy(cfg, child, shutdown).await?;
    let id = register(cfg, child, shutdown).await?;
    backoff.reset(); // successful registration = healthy cycle
    watch(cfg, child, &id, shutdown).await
}
```

### Exit reasons

```rust
enum ExitReason {
    Shutdown,         // SIGTERM/SIGINT received
    Crashed,          // backend process exited unexpectedly
    Expired,          // kernel evicted dog (heartbeat 404)
    Mismatch,         // /v1/models returned wrong model
    Fatal(String),    // permanent error — name collision, bad config
}
```

## Config Format

```toml
# /etc/cynic/node-qwen.toml

[kernel]
url = "http://<TAILSCALE_CORE>:3030"
api_key_env = "CYNIC_API_KEY"               # env var name, never literal

[dog]
name = "qwen35-9b-gpu"
model = "qwen3.5:9b-q4_K_M"
base_url = "http://localhost:8080/v1"
context_size = 8192                          # optional, default 4096
timeout_secs = 60                            # optional, default 60

[process]
command = ["llama-server", "-m", "/models/qwen3.5-9b-q4_K_M.gguf",
           "--port", "8080", "--ctx-size", "8192"]
working_dir = "/opt/llama"                   # optional
stop_timeout_secs = 10                       # SIGTERM → wait → SIGKILL

[process.env]                                # injected into child, optional
LLAMA_LOG_VERBOSITY = "0"

[restart]
max_attempts = 5                             # consecutive failures → exit(1)
initial_delay_secs = 2
max_delay_secs = 120                         # backoff cap
min_uptime_secs = 10                         # resets failure counter

[health]
interval_secs = 15
timeout_secs = 5
max_failures = 3                             # consecutive failures → kill backend
startup_timeout_secs = 120                   # max wait for first healthy response
```

### Config validation (fail fast at startup)

- `api_key_env` must resolve to a non-empty env var
- `command` must be non-empty, `command[0]` should be in PATH (warning, not error — might be absolute)
- `name` must be 1–64 chars (kernel constraint)
- `base_url` must parse as valid URL
- `stop_timeout_secs` > 0
- `max_attempts` > 0
- `startup_timeout_secs` > `health.timeout_secs`

### Derived values

- `health_url` = strip `/v1` from `dog.base_url`, append `/health`
- `models_url` = `dog.base_url` + `/models`
- `heartbeat_interval` = `kernel_ttl / 3` = 40s (TTL=120s, check every 30s on kernel → heartbeat at 40s gives ~3 beats per TTL window)

## Three Concerns

### 1. Supervise (`supervise.rs`)

**Spawn:**
```rust
fn spawn_backend(config: &ProcessConfig) -> io::Result<Box<dyn ChildWrapper>> {
    let mut wrap = CommandWrap::with_new(&config.command[0], |cmd| {
        cmd.args(&config.command[1..])
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        if let Some(dir) = &config.working_dir {
            cmd.current_dir(dir);
        }
        for (k, v) in &config.env {
            cmd.env(k, v);
        }
    });

    #[cfg(unix)]
    let wrap = wrap.wrap(ProcessGroup::leader());
    #[cfg(windows)]
    let wrap = wrap.wrap(JobObject);

    wrap.wrap(KillOnDrop).spawn()
}
```

**Spawn with retries:**
```rust
fn spawn_with_retries(config, max: u32) -> Result<Child, SpawnError> {
    for attempt in 1..=max {
        match spawn_backend(&config.process) {
            Ok(child) => return Ok(child),
            Err(e) => {
                tracing::error!("spawn attempt {attempt}/{max} failed: {e}");
                if attempt < max {
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }
    }
    Err(SpawnError::Exhausted)
}
```

**Graceful stop:**
```rust
async fn graceful_stop(child: &mut Box<dyn ChildWrapper>, timeout_secs: u64) {
    #[cfg(unix)]
    let _ = child.signal(libc::SIGTERM);
    // Windows: KillOnDrop + JobObject handles termination

    match tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        Box::into_pin(child.wait()),
    ).await {
        Ok(_) => { /* clean exit */ }
        Err(_) => {
            tracing::warn!("backend did not stop within {timeout_secs}s, killing");
            let _ = child.start_kill();
            let _ = Box::into_pin(child.wait()).await;
        }
    }
}
```

**Backoff:**
```rust
struct Backoff {
    attempt: u32,
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    min_uptime: Duration,
    last_start: Instant,
}

impl Backoff {
    fn record_start(&mut self) { self.last_start = Instant::now(); }

    fn reset_if_stable(&mut self) {
        if self.last_start.elapsed() >= self.min_uptime {
            self.attempt = 0;
        }
    }

    fn exhausted(&self) -> bool { self.attempt >= self.max_attempts }

    fn next_delay(&mut self) -> Duration {
        self.attempt += 1;
        let delay = self.initial_delay * 2u32.saturating_pow(self.attempt - 1);
        delay.min(self.max_delay)
    }

    async fn wait_or_shutdown(&mut self, shutdown: &CancellationToken) {
        let delay = self.next_delay();
        tracing::info!("backing off {delay:?} (attempt {}/{})", self.attempt, self.max_attempts);
        select! {
            _ = tokio::time::sleep(delay) => {}
            _ = shutdown.cancelled() => {}
        }
    }
}
```

### 2. Announce (`announce.rs`)

**Register (races crash + shutdown):**
```rust
async fn register(cfg, child, shutdown) -> Result<String, ExitReason> {
    let payload = json!({
        "name": cfg.dog.name,
        "base_url": cfg.dog.base_url,
        "model": cfg.dog.model,
        "context_size": cfg.dog.context_size,
        "timeout_secs": cfg.dog.timeout_secs,
    });

    let mut backoff = Backoff::new(&cfg.restart);
    loop {
        select! {
            result = try_register(&cfg.kernel, &payload) => match result {
                Ok(resp) => {
                    tracing::info!("registered as {} (roster: {})", resp.dog_id, resp.roster_size);
                    return Ok(resp.dog_id);
                }
                Err(e) if e.is_collision() => return Err(Fatal(format!("name collision: {}", cfg.dog.name))),
                Err(e) if e.is_permanent() => return Err(Fatal(e.to_string())),
                Err(e) => {
                    tracing::warn!("registration failed: {e}, retrying");
                    backoff.wait_or_shutdown(shutdown).await;
                }
            },
            _ = child.wait() => return Err(Crashed),
            _ = shutdown.cancelled() => return Err(Shutdown),
        }
    }
}
```

**Heartbeat (inside watch select!):**
```rust
async fn send_heartbeat(cfg, dog_id) -> HeartbeatResult {
    let url = format!("{}/dogs/{}/heartbeat", cfg.kernel.url, dog_id);
    match http_post(&url, &cfg.kernel.api_key).await {
        Ok(200) => HeartbeatResult::Alive,
        Ok(404) => HeartbeatResult::Expired,
        Ok(s)   => HeartbeatResult::Error(format!("unexpected status {s}")),
        Err(e)  => HeartbeatResult::Error(e.to_string()),
    }
}
```

**Deregister (best-effort, never blocks exit):**
```rust
async fn try_deregister(cfg, dog_id: &str) {
    let url = format!("{}/dogs/{}", cfg.kernel.url, dog_id);
    match tokio::time::timeout(
        Duration::from_secs(5),
        http_delete(&url, &cfg.kernel.api_key),
    ).await {
        Ok(Ok(_)) => tracing::info!("deregistered {dog_id}"),
        _ => tracing::warn!("deregister failed for {dog_id}, TTL will clean up"),
    }
}
```

### 3. Verify (`verify.rs`)

**Health check:**
```rust
async fn check_health(cfg) -> bool {
    let url = derive_health_url(&cfg.dog.base_url);
    match tokio::time::timeout(
        Duration::from_secs(cfg.health.timeout_secs),
        reqwest::get(&url),
    ).await {
        Ok(Ok(resp)) if resp.status().is_success() => true,
        _ => false,
    }
}
```

**Identity check:**
```rust
async fn check_identity(cfg) -> IdentityResult {
    let url = format!("{}/models", cfg.dog.base_url);
    match reqwest::get(&url).await {
        Ok(resp) => {
            let body: Value = resp.json().await?;
            let models = body["data"].as_array();
            if let Some(models) = models {
                let loaded = models.iter()
                    .filter_map(|m| m["id"].as_str())
                    .collect::<Vec<_>>();
                if loaded.iter().any(|id| id.contains(&cfg.dog.model)) {
                    IdentityResult::Match
                } else {
                    IdentityResult::Mismatch {
                        expected: cfg.dog.model.clone(),
                        actual: loaded.join(", "),
                    }
                }
            } else {
                IdentityResult::Unknown
            }
        }
        Err(_) => IdentityResult::Unreachable,
    }
}
```

**Identity mismatch action (K15 compliance):** `Mismatch` returned from watch loop → outer loop sets `needs_spawn = true` → backend restarted. If mismatch persists across restarts, `backoff.exhausted()` → `exit(1)`. This is the acting consumer: wrong model = restart. Persistent wrong model = circuit-open.

### Watch loop

```rust
async fn watch(cfg, child, dog_id, shutdown) -> ExitReason {
    let mut heartbeat_tick = interval(Duration::from_secs(40));
    let mut health_tick = interval(Duration::from_secs(cfg.health.interval_secs));
    let mut verify_tick = interval(Duration::from_secs(60));
    let mut health_failures: u32 = 0;

    loop {
        select! {
            _ = heartbeat_tick.tick() => {
                match send_heartbeat(cfg, dog_id).await {
                    Alive => {}
                    Expired => return ExitReason::Expired,
                    Error(e) => tracing::warn!("heartbeat error: {e}"),
                }
            }

            _ = health_tick.tick() => {
                if check_health(cfg).await {
                    health_failures = 0;
                } else {
                    health_failures += 1;
                    tracing::warn!("health check failed ({health_failures}/{})", cfg.health.max_failures);
                    if health_failures >= cfg.health.max_failures {
                        tracing::error!("backend unresponsive, killing");
                        #[cfg(unix)]
                        let _ = child.signal(libc::SIGTERM);
                        // child.wait() will fire → Crashed
                    }
                }
            }

            _ = verify_tick.tick() => {
                match check_identity(cfg).await {
                    Match => {}
                    Mismatch { expected, actual } => {
                        tracing::error!("model mismatch: expected {expected}, got {actual}");
                        return ExitReason::Mismatch;
                    }
                    Unknown | Unreachable => {
                        // Don't act on transient verify failures — health check handles liveness
                    }
                }
            }

            status = Box::into_pin(child.wait()) => {
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

## Wait Healthy (startup probe)

```rust
async fn wait_healthy(cfg, child, shutdown) -> Result<(), ExitReason> {
    let deadline = Instant::now() + Duration::from_secs(cfg.health.startup_timeout_secs);
    let mut tick = interval(Duration::from_secs(2));

    loop {
        select! {
            _ = tick.tick() => {
                if check_health(cfg).await {
                    tracing::info!("backend healthy");
                    return Ok(());
                }
                if Instant::now() >= deadline {
                    tracing::error!("startup health timeout after {}s", cfg.health.startup_timeout_secs);
                    return Err(Crashed); // kill + restart
                }
            }
            _ = Box::into_pin(child.wait()) => return Err(Crashed),
            _ = shutdown.cancelled() => return Err(Shutdown),
        }
    }
}
```

## Error Classification

| Error | Class | Source | Action |
|---|---|---|---|
| Binary not found | Permanent | `spawn_backend()` | Exit after 3 |
| `api_key_env` missing | Permanent | Config validation | Exit at startup |
| Name collision (409) | Permanent | `register()` | Exit immediately |
| Bad config (parse fail) | Permanent | Config validation | Exit at startup |
| Calibration fail (422) | Semi-permanent | `register()` | Retry with backoff, exit after `max_attempts` |
| Calibration timeout (504) | Transient | `register()` | Retry with backoff |
| Kernel unreachable | Transient | `register()` / `heartbeat()` | Retry indefinitely (backend still useful) |
| Backend crash | Transient | `child.wait()` | Restart with backoff |
| Health timeout | Transient | `check_health()` | 3 consecutive → kill → restart |
| Model mismatch | Transient | `check_identity()` | Restart backend (reload model) |
| Heartbeat 404 | Expected | `send_heartbeat()` | Re-register (no restart) |

## File Structure

```
cynic-node/
├── Cargo.toml
├── src/
│   ├── main.rs        # CLI (clap), config load, outer loop, signal handler
│   ├── config.rs      # TOML structs, validation, derived values
│   ├── supervise.rs   # spawn_backend(), graceful_stop(), spawn_with_retries(), Backoff
│   ├── announce.rs    # register(), send_heartbeat(), try_deregister()
│   └── verify.rs      # check_health(), check_identity(), wait_healthy()
```

## Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread", "process", "time", "signal", "io-util"] }
tokio-util = "0.7"            # CancellationToken
reqwest = { version = "0.13", default-features = false, features = ["json", "native-tls"] }
process-wrap = { version = "9.1", features = ["tokio1"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

Platform-specific (compile-time only):
```toml
[target.'cfg(unix)'.dependencies]
libc = "0.2"                  # SIGTERM constant (15)
```

No `backoff` crate (RUSTSEC-2025-0012) — manual implementation in `Backoff` struct.
No `nix` — use `libc::SIGTERM` directly (just the constant, no FFI).

## Cross-Platform

| Concern | Linux | Windows |
|---|---|---|
| Orphan safety | `ProcessGroup::leader()` | `JobObject` |
| Graceful stop | `child.signal(libc::SIGTERM)` | `KillOnDrop` (TerminateProcess via JobObject) |
| Signal handling | `tokio::signal::unix::signal(SIGTERM)` | `tokio::signal::ctrl_c()` |
| Log output | stdout → journald | stdout → Event Log / console |

Windows graceful stop limitation: `TerminateProcess` is not graceful — it's equivalent to SIGKILL. For llama-server on Windows, this is acceptable (no state to flush). If future backends need graceful Windows stop, Phase C can add `GenerateConsoleCtrlEvent(CTRL_C_EVENT)`.

## Heartbeat Timing

- Kernel TTL: 120s
- Kernel TTL checker: runs every 30s
- Node heartbeat interval: 40s
- Worst case: heartbeat sent at T=0, next at T=40. TTL checker runs at T=30 (TTL=120-30=90s remaining), then T=60 (TTL=120-60=60s remaining). Even missing 2 consecutive heartbeats (80s), the dog survives (120-80=40s remaining, next TTL check at T=90 catches it alive at 40s). Missing 3 heartbeats (120s) = eviction.
- Safety margin: 3 missed heartbeats before eviction. Practical failure window: ~2 minutes of network partition.

## Testing Strategy

### Unit tests (in-module `#[cfg(test)]`)

- `config.rs`: Parse valid/invalid TOML, validate constraints, derive health_url
- `supervise.rs`: Backoff progression (delays, exhaustion, reset after min_uptime)
- `announce.rs`: Heartbeat result parsing, deregister timeout
- `verify.rs`: Identity match/mismatch logic, health_url derivation

### Integration tests (`tests/`)

- **Mock kernel**: Axum server on `127.0.0.1:0` implementing `/dogs/register`, `/dogs/{id}/heartbeat`, `/dogs/{id}` (DELETE). Tests full register→heartbeat→deregister cycle.
- **Mock backend**: Axum server on `127.0.0.1:0` implementing `/health`, `/v1/models`, `/v1/chat/completions`. Tests health check, identity verification, calibration.
- **Process supervision**: Spawn `sleep 999` (Unix) or `timeout /t 999` (Windows) as a test backend. Verify spawn, graceful stop (SIGTERM + wait), forced kill.
- **Crash restart**: Spawn `false` (exits immediately). Verify backoff timing, max_attempts circuit-break.
- **Heartbeat 404 → re-register**: Mock kernel returns 200 for first N heartbeats, then 404. Verify node re-registers without restarting backend.
- **Shutdown during phases**: Send cancellation during wait_healthy, during register, during watch. Verify clean exit in all cases.

### End-to-end (manual, with real kernel)

- Deploy on cynic-gpu with real llama-server + real kernel
- Kill llama-server → verify restart + re-registration
- Stop kernel → verify node keeps backend alive, re-registers when kernel returns
- Load wrong model → verify identity mismatch → restart

## Deployment

### systemd (Linux)

```ini
[Unit]
Description=CYNIC Node — %i
After=network-online.target
Wants=network-online.target

[Service]
ExecStart=/usr/local/bin/cynic-node --config /etc/cynic/node-%i.toml
EnvironmentFile=-%h/.config/cynic/env
Restart=on-failure
RestartSec=10
StartLimitBurst=3
StartLimitIntervalSec=300

[Install]
WantedBy=multi-user.target
```

Usage: `systemctl --user enable --now cynic-node@qwen35-9b-gpu`

The node handles its own restart logic (backoff, max_attempts). systemd is the outer circuit-breaker: if the node itself crashes or exits(1) repeatedly, systemd stops trying.

### Windows

```
cynic-node.exe --config C:\cynic\node-qwen.toml
```

Run as a Windows Service via `sc create` or NSSM, or as a startup task replacing the current schtask.
