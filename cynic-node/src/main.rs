// Binary crate test exemption — same pattern as cynic-kernel lib.rs
#![cfg_attr(test, allow(dead_code, clippy::unwrap_used, clippy::expect_used))]

mod announce;
mod config;
mod supervise;
mod verify;

use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use process_wrap::tokio::ChildWrapper;
use reqwest::Client;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::config::Config;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "cynic-node", about = "CYNIC inference backend supervisor")]
struct Cli {
    /// Path to node config file
    #[arg(short, long)]
    config: String,
}

// ── ExitReason ────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum ExitReason {
    /// SIGTERM / SIGINT received — clean shutdown requested.
    Shutdown,
    /// Backend process exited unexpectedly or failed health checks.
    Crashed,
    /// Kernel evicted this dog (heartbeat returned 404).
    Expired,
    /// Wrong model loaded — backend identity check failed.
    Mismatch,
    /// Permanent error that cannot be recovered from.
    Fatal(String),
}

// ── watch ─────────────────────────────────────────────────────────────────────

/// Main supervision loop: heartbeat, health, identity, child-exit, shutdown.
///
/// The `child.wait()` future is recreated on each iteration and pinned BEFORE
/// the `select!` call.  When any other branch wins, Rust drops `child_wait` and
/// releases the `&mut child` borrow, allowing the next iteration to re-borrow.
async fn watch(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    dog_id: &str,
    shutdown: &CancellationToken,
) -> ExitReason {
    let health_url = config::derive_health_url(&cfg.dog.base_url);
    let models_url = config::derive_models_url(&cfg.dog.base_url);
    let mut heartbeat_tick =
        tokio::time::interval(Duration::from_secs(cfg.kernel.heartbeat_interval_secs));
    let mut health_tick = tokio::time::interval(Duration::from_secs(cfg.health.interval_secs));
    let mut verify_tick =
        tokio::time::interval(Duration::from_secs(cfg.health.verify_interval_secs));
    let mut health_failures: u32 = 0;

    loop {
        // Pin child.wait() OUTSIDE select! so the &mut borrow is released when
        // any other branch wins (C2 fix: no borrow-across-await conflict).
        let child_wait = child.wait();
        tokio::pin!(child_wait);

        tokio::select! {
            _ = heartbeat_tick.tick() => {
                if let Err(r) = on_heartbeat_tick(client, cfg, dog_id).await {
                    return r;
                }
            }
            _ = health_tick.tick() => {
                if let Some(r) = on_health_tick(client, cfg, &health_url, &mut health_failures).await {
                    return r;
                }
            }
            _ = verify_tick.tick() => {
                if let Some(r) = on_verify_tick(client, cfg, &models_url).await {
                    return r;
                }
            }
            status = &mut child_wait => {
                tracing::error!("backend process exited unexpectedly: {status:?}");
                return ExitReason::Crashed;
            }
            _ = shutdown.cancelled() => return ExitReason::Shutdown,
        }
    }
}

/// Handle one heartbeat tick. Returns Err(ExitReason) if we should exit.
async fn on_heartbeat_tick(client: &Client, cfg: &Config, dog_id: &str) -> Result<(), ExitReason> {
    match announce::send_heartbeat(client, &cfg.kernel.url, &cfg.kernel.api_key, dog_id).await {
        announce::HeartbeatResult::Alive => Ok(()),
        announce::HeartbeatResult::Expired => Err(ExitReason::Expired),
        announce::HeartbeatResult::Error(e) => {
            tracing::warn!("heartbeat error: {e}");
            Ok(())
        }
    }
}

/// Handle one health tick. Returns Some(ExitReason) if we should exit.
async fn on_health_tick(
    client: &Client,
    cfg: &Config,
    health_url: &str,
    health_failures: &mut u32,
) -> Option<ExitReason> {
    if verify::check_health(client, health_url, cfg.health.timeout_secs).await {
        *health_failures = 0;
        None
    } else {
        *health_failures += 1;
        tracing::warn!(
            "health fail ({}/{})",
            health_failures,
            cfg.health.max_failures
        );
        if *health_failures >= cfg.health.max_failures {
            tracing::error!(
                "backend unresponsive after {} failures — will restart",
                cfg.health.max_failures
            );
            // Return Crashed — the main loop is responsible for graceful_stop.
            // We cannot call graceful_stop here while child_wait holds &mut child.
            return Some(ExitReason::Crashed);
        }
        None
    }
}

/// Handle one identity-verification tick. Returns Some(ExitReason) if we should exit.
async fn on_verify_tick(client: &Client, cfg: &Config, models_url: &str) -> Option<ExitReason> {
    match verify::check_identity(client, models_url, &cfg.dog.model, cfg.health.timeout_secs).await
    {
        verify::IdentityResult::Match => None,
        verify::IdentityResult::Mismatch { expected, actual } => {
            tracing::error!("model mismatch: expected {expected}, got {actual}");
            Some(ExitReason::Mismatch)
        }
        // Unreachable / Unknown are transient — health check will catch persistent failure
        _ => None,
    }
}

// ── wait_healthy ──────────────────────────────────────────────────────────────

/// Poll the backend health endpoint until it responds 2xx or the deadline is exceeded.
///
/// Races against child exit and shutdown to avoid waiting forever when the
/// process has already died.
async fn wait_healthy(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    shutdown: &CancellationToken,
) -> Result<(), ExitReason> {
    let health_url = config::derive_health_url(&cfg.dog.base_url);
    let deadline =
        tokio::time::Instant::now() + Duration::from_secs(cfg.health.startup_timeout_secs);
    let mut tick = tokio::time::interval(Duration::from_secs(2));

    loop {
        let child_wait = child.wait();
        tokio::pin!(child_wait);

        tokio::select! {
            _ = tick.tick() => {
                if verify::check_health(client, &health_url, cfg.health.timeout_secs).await {
                    tracing::info!("backend healthy — proceeding to registration");
                    return Ok(());
                }
                if tokio::time::Instant::now() >= deadline {
                    tracing::error!(
                        "startup health timeout after {}s",
                        cfg.health.startup_timeout_secs
                    );
                    // Return Crashed — the main loop is responsible for graceful_stop.
                    // We cannot call graceful_stop here while child_wait holds &mut child.
                    return Err(ExitReason::Crashed);
                }
                tracing::debug!("backend not yet healthy, retrying...");
            }
            _ = &mut child_wait => {
                tracing::error!("backend exited during startup health check");
                return Err(ExitReason::Crashed);
            }
            _ = shutdown.cancelled() => return Err(ExitReason::Shutdown),
        }
    }
}

// ── register_with_kernel ──────────────────────────────────────────────────────

/// Attempt to register this node as a Dog with the kernel.
///
/// Calibration failures are counted separately from transient errors (M1 fix):
/// - Collision (409)       → Fatal immediately (duplicate name)
/// - CalibrationFail (422) → count; exit after `max_calibration_attempts`
/// - Transient             → retry indefinitely (backend is still useful)
///
/// Races against child exit and shutdown.
async fn register_with_kernel(
    client: &Client,
    cfg: &Config,
    child: &mut Box<dyn ChildWrapper>,
    shutdown: &CancellationToken,
) -> Result<String, ExitReason> {
    let payload = build_registration_payload(cfg);
    let max_calibration_attempts: u32 = 3;
    let mut calibration_attempts: u32 = 0;

    loop {
        let child_wait = child.wait();
        tokio::pin!(child_wait);

        tokio::select! {
            result = announce::try_register(client, &cfg.kernel.url, &cfg.kernel.api_key, &payload) => {
                match result {
                    Ok(resp) => {
                        tracing::info!(
                            dog_id = %resp.dog_id,
                            roster_size = resp.roster_size,
                            "registered with kernel"
                        );
                        return Ok(resp.dog_id);
                    }
                    Err(announce::RegisterError::Collision) => {
                        return Err(ExitReason::Fatal(format!(
                            "registration collision: a dog named '{}' is already registered",
                            cfg.dog.name
                        )));
                    }
                    Err(announce::RegisterError::CalibrationFail(body)) => {
                        calibration_attempts += 1;
                        tracing::warn!(attempt = calibration_attempts, max = max_calibration_attempts, "calibration failed: {body}");
                        if calibration_attempts >= max_calibration_attempts {
                            return Err(ExitReason::Fatal(format!(
                                "calibration failed after {max_calibration_attempts} attempts: {body}"
                            )));
                        }
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                    Err(announce::RegisterError::Transient(msg)) => {
                        // Transient errors are always retried — kernel may be momentarily unavailable.
                        tracing::warn!("transient registration error (will retry): {msg}");
                        tokio::time::sleep(Duration::from_secs(10)).await;
                    }
                }
            }
            _ = &mut child_wait => {
                tracing::error!("backend exited during registration");
                return Err(ExitReason::Crashed);
            }
            _ = shutdown.cancelled() => return Err(ExitReason::Shutdown),
        }
    }
}

/// Build the JSON payload for POST /dogs/register from the config.
fn build_registration_payload(cfg: &Config) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "name":         cfg.dog.name,
        "model":        cfg.dog.model,
        "base_url":     cfg.dog.base_url,
        "context_size": cfg.dog.context_size,
        "timeout_secs": cfg.dog.timeout_secs,
    });
    if let Some(ref key) = cfg.dog.api_key {
        payload["api_key"] = serde_json::Value::String(key.clone());
    }
    payload
}

// ── run_lifecycle ─────────────────────────────────────────────────────────────

/// Run one full lifecycle: wait_healthy → register → watch.
///
/// Returns the exit reason and the dog_id if registration succeeded before exit.
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

// ── Signal handler ────────────────────────────────────────────────────────────

/// Install OS signal handlers and cancel `shutdown` when a signal arrives.
///
/// On Unix: listens for SIGTERM and SIGINT.
/// On Windows: listens for Ctrl-C.
fn install_signal_handler(shutdown: CancellationToken) {
    #[cfg(unix)]
    tokio::spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Err(e) => {
                // WHY: If we cannot install SIGTERM, log the failure but continue.
                // SIGINT (ctrl-c) is still available. The process can still be stopped
                // via SIGKILL if needed. Aborting here would be worse than degraded mode.
                tracing::warn!("SIGTERM handler install failed ({e}) — only SIGINT will work");
                tokio::signal::ctrl_c().await.ok();
            }
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = sigterm.recv() => tracing::info!("SIGTERM received — shutting down"),
                    _ = tokio::signal::ctrl_c() => tracing::info!("SIGINT received — shutting down"),
                }
            }
        }
        shutdown.cancel();
    });

    #[cfg(windows)]
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Ctrl-C received — shutting down");
        shutdown.cancel();
    });
}

// ── Supervision loop ──────────────────────────────────────────────────────────

/// The main supervision loop.  Returns `ExitCode::SUCCESS` on clean shutdown,
/// `ExitCode::FAILURE` on unrecoverable errors.
async fn run_node(client: &Client, cfg: &Config, shutdown: &CancellationToken) -> ExitCode {
    let mut state = NodeState {
        needs_spawn: true,
        child: None,
        backoff: supervise::Backoff::from_config(&cfg.restart),
        dog_id: None,
    };

    let exit_code = loop {
        if state.needs_spawn {
            if let Some(id) = state.dog_id.take() {
                deregister(client, cfg, &id).await;
            }
            match spawn_backend(cfg).await {
                Ok(c) => {
                    state.child = Some(c);
                    state.backoff.record_start();
                }
                Err(e) => {
                    tracing::error!("spawn failed: {e}");
                    break ExitCode::FAILURE;
                }
            }
        }

        let Some(c) = state.child.as_mut() else {
            tracing::error!("internal: child is None after spawn");
            break ExitCode::FAILURE;
        };

        let (reason, id) = run_lifecycle(client, cfg, c, shutdown, &mut state.backoff).await;
        state.dog_id = id;

        match handle_exit_reason(reason, cfg, &mut state, shutdown, client).await {
            LoopAction::Continue => {}
            LoopAction::Break(code) => break code,
        }
    };

    tracing::info!("cynic-node stopped");
    exit_code
}

/// Action the supervision loop should take after handling an exit reason.
enum LoopAction {
    Continue,
    Break(ExitCode),
}

/// Mutable state threaded through the supervision loop.
#[derive(Debug)]
struct NodeState {
    needs_spawn: bool,
    child: Option<Box<dyn ChildWrapper>>,
    backoff: supervise::Backoff,
    dog_id: Option<String>,
}

/// Process an ExitReason and update loop state. Returns the next loop action.
async fn handle_exit_reason(
    reason: ExitReason,
    cfg: &Config,
    state: &mut NodeState,
    shutdown: &CancellationToken,
    client: &Client,
) -> LoopAction {
    match reason {
        ExitReason::Shutdown => {
            if let Some(id) = state.dog_id.take() {
                deregister(client, cfg, &id).await;
            }
            stop_child(&mut state.child, cfg).await;
            LoopAction::Break(ExitCode::SUCCESS)
        }
        ExitReason::Expired => {
            tracing::warn!("kernel expired this dog — will re-register without respawn");
            state.dog_id = None;
            state.needs_spawn = false;
            LoopAction::Continue
        }
        ExitReason::Crashed => {
            state.dog_id = None;
            state.needs_spawn = true;
            state.backoff.reset_if_stable();
            if state.backoff.exhausted() {
                tracing::error!(
                    "max restart attempts ({}) reached — giving up",
                    cfg.restart.max_attempts
                );
                stop_child(&mut state.child, cfg).await;
                return LoopAction::Break(ExitCode::FAILURE);
            }
            tracing::info!("backend crashed — waiting before restart");
            state.backoff.wait_or_shutdown(shutdown).await;
            LoopAction::Continue
        }
        ExitReason::Mismatch => {
            if let Some(id) = state.dog_id.take() {
                deregister(client, cfg, &id).await;
            }
            stop_child(&mut state.child, cfg).await;
            state.needs_spawn = true;
            LoopAction::Continue
        }
        ExitReason::Fatal(e) => {
            tracing::error!("fatal: {e}");
            if let Some(id) = state.dog_id.take() {
                deregister(client, cfg, &id).await;
            }
            stop_child(&mut state.child, cfg).await;
            LoopAction::Break(ExitCode::FAILURE)
        }
    }
}

/// Best-effort graceful stop of the child process.
async fn stop_child(child: &mut Option<Box<dyn ChildWrapper>>, cfg: &Config) {
    if let Some(c) = child.as_mut() {
        let _ = supervise::graceful_stop(c, cfg.process.stop_timeout_secs).await;
    }
}

/// Best-effort deregistration of a dog from the kernel.
async fn deregister(client: &Client, cfg: &Config, dog_id: &str) {
    announce::try_deregister(client, &cfg.kernel.url, &cfg.kernel.api_key, dog_id).await;
}

/// Spawn the backend process with retries.
async fn spawn_backend(cfg: &Config) -> std::io::Result<Box<dyn ChildWrapper>> {
    tracing::info!("spawning backend: {:?}", cfg.process.command);
    supervise::spawn_with_retries(
        &cfg.process.command,
        cfg.process.working_dir.as_deref(),
        &cfg.process.env,
        3,
    )
    .await
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let cfg = match config::load(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("config error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.health.timeout_secs))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("HTTP client init failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let shutdown = CancellationToken::new();
    install_signal_handler(shutdown.clone());

    run_node(&client, &cfg, &shutdown).await
}
