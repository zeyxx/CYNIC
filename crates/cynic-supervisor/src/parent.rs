//! Supervised parent: one process per role holding the child handle.
//!
//! The parent owns the `KillOnDrop` handle for its whole lifetime, so the
//! child dies with the parent and never orphans. Restart decisions come from
//! the canonical `decide_restart`; restart counts persist across parent
//! restarts; live state is published through an atomic status file that
//! `cynic supervise observe` reads. See
//! `docs/design/supervised-parent-scope.md` (spec + adversarial review).

use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cynic_protocol::supervision::{
    decide_restart, ParentStatusEnvelope, RestartDecision, RestartPolicy, SupervisorObservation,
    SupervisorState,
};
use process_wrap::tokio::ChildWrapper;

use crate::{graceful_stop, spawn_backend};

/// How often the parent refreshes the status file heartbeat.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// Takeover grace: a lock older than this with a dead holder may be taken.
const TAKEOVER_GRACE: Duration = Duration::from_secs(10);
/// Command-file poll step inside restart-delay waits.
const CMD_POLL_STEP: Duration = Duration::from_secs(1);

/// One poll step that also listens for shutdown signals (Unix).
/// Returns `true` when SIGTERM/SIGINT arrived during the step, so the parent
/// stops the child gracefully instead of dying and orphaning it.
#[cfg(unix)]
async fn wait_step(
    term: Option<&mut tokio::signal::unix::Signal>,
    int: Option<&mut tokio::signal::unix::Signal>,
) -> bool {
    async fn recv_or_pending(sig: Option<&mut tokio::signal::unix::Signal>) {
        if let Some(sig) = sig {
            let _ = sig.recv().await;
        } else {
            std::future::pending::<()>().await;
        }
    }
    tokio::select! {
        _ = recv_or_pending(term) => true,
        _ = recv_or_pending(int) => true,
        _ = tokio::time::sleep(CMD_POLL_STEP) => false,
    }
}

/// Non-Unix step: plain sleep, no signal trap (platform scope, spec A10).
#[cfg(not(unix))]
async fn wait_step(_term: (), _int: ()) -> bool {
    tokio::time::sleep(CMD_POLL_STEP).await;
    false
}

/// Parent configuration for one role. The command argv is declared by the
/// caller (systemd unit); the parent never invents it.
#[derive(Debug, Clone)]
pub struct RoleRunConfig {
    pub role_id: String,
    pub command: Vec<String>,
    pub working_dir: Option<String>,
    pub env: HashMap<String, String>,
    pub policy: RestartPolicy,
    pub stop_grace_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub state_dir: PathBuf,
    pub status_file: PathBuf,
}

/// How a parent run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleRunOutcome {
    /// Child exited cleanly; parent stops without restarting.
    Clean,
    /// Restart bound reached; parent stops with the decision reason.
    GiveUp { reason: String },
}

/// Single-parent lock: `(pid, epoch_uuid, started_at_ms)`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ParentLock {
    pid: u32,
    epoch: String,
    started_at_ms: u128,
}

/// Persisted restart history across parent restarts (spec amendment A4).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct RestartHistory {
    count: u32,
    window_start_ms: u128,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|t| t.as_millis())
        .unwrap_or(0)
}

fn lock_path(state_dir: &Path, role_id: &str) -> PathBuf {
    state_dir.join(format!("supervise-{role_id}.lock"))
}

fn cmd_path(state_dir: &Path, role_id: &str) -> PathBuf {
    state_dir.join(format!("supervise-{role_id}.cmd.json"))
}

/// Consume a pending stop request exactly once (atomic delete after read).
/// Unknown commands and wrong-schema files are left for a future step,
/// never executed.
fn consume_stop_request(state_dir: &Path, role_id: &str) -> bool {
    let path = cmd_path(state_dir, role_id);
    let raw = match std::fs::read(&path) {
        Ok(raw) => raw,
        Err(_) => return false,
    };
    let cmd: cynic_protocol::supervision::ParentCommand = match serde_json::from_slice(&raw) {
        Ok(cmd) => cmd,
        Err(_) => return false,
    };
    if cmd.schema != cynic_protocol::supervision::PARENT_COMMAND_FILE_SCHEMA
        || cmd.command != cynic_protocol::supervision::PARENT_COMMAND_STOP
    {
        return false;
    }
    let _ = std::fs::remove_file(&path);
    true
}

/// Refresh lock liveness so `is_role_parented` keeps reporting a running
/// parent (its freshness check reads lock mtime). Called on every status
/// heartbeat; the lock content is unchanged, only mtime moves.
fn touch_lock(state_dir: &Path, role_id: &str) {
    let path = lock_path(state_dir, role_id);
    if let Ok(raw) = std::fs::read(&path) {
        let _ = write_atomic(&path, &raw);
    }
}

fn restarts_path(state_dir: &Path, role_id: &str) -> PathBuf {
    state_dir.join(format!("supervise-{role_id}.restarts.json"))
}

/// Atomic file write: temp + rename, fsync'd before the rename so a crash
/// never leaves a half-written file behind.
fn write_atomic(path: &Path, payload: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(payload)?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn holder_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).is_dir()
}

#[cfg(not(target_os = "linux"))]
fn holder_alive(pid: u32) -> bool {
    // No /proc outside Linux: liveness unknown, fall back to mtime grace.
    let _ = pid;
    false
}

/// Default state directory for parent locks and restart history
/// (Linux instantiation; override per host scope).
pub const DEFAULT_SUPERVISE_STATE_DIR: &str = "/var/lib/cynic/supervise";
/// Declared role id for the local inference backend under a parent.
pub const INFERENCE_ROLE_ID: &str = "inference";

/// Returns `true` when a live parent currently holds `role_id`: the lock
/// file exists, parses, names a live holder, and is freshly heartbeated.
/// Anything else (absent, corrupt, dead holder, stale) is not parented —
/// callers report explicit unknown, never assume custody.
pub fn is_role_parented(state_dir: &Path, role_id: &str) -> bool {
    let path = lock_path(state_dir, role_id);
    let raw = match std::fs::read(&path) {
        Ok(raw) => raw,
        Err(_) => return false,
    };
    let lock: ParentLock = match serde_json::from_slice(&raw) {
        Ok(lock) => lock,
        Err(_) => return false,
    };
    if !holder_alive(lock.pid) {
        return false;
    }
    let fresh = std::fs::metadata(&path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .map(|d| d < HEARTBEAT_INTERVAL + HEARTBEAT_INTERVAL)
        .unwrap_or(false);
    fresh
}

/// Acquire the single-parent lock or fail with a named error.
/// A lock whose holder is dead past the takeover grace is taken over;
/// the takeover is reported to the caller for the first observation.
fn acquire_lock(state_dir: &Path, role_id: &str) -> io::Result<Option<String>> {
    let path = lock_path(state_dir, role_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let lock = ParentLock {
        pid: std::process::id(),
        epoch: format!("{}-{}", std::process::id(), now_ms()),
        started_at_ms: now_ms(),
    };
    let payload =
        serde_json::to_vec(&lock).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(&payload)?;
            Ok(None)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            let raw = std::fs::read(&path)?;
            let old: ParentLock = serde_json::from_slice(&raw)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let mtime_age_ms = std::fs::metadata(&path)?
                .modified()
                .ok()
                .and_then(|t| t.elapsed().ok())
                .map(|d| d.as_millis())
                .unwrap_or(u128::MAX);
            let dead = !holder_alive(old.pid);
            let stale = mtime_age_ms > TAKEOVER_GRACE.as_millis();
            if dead && stale {
                write_atomic(&path, &payload)?;
                Ok(Some(format!("supervisor_lock_takeover:{}", old.epoch)))
            } else {
                Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("supervisor_parent_holds_role:{role_id} (pid {})", old.pid),
                ))
            }
        }
        Err(e) => Err(e),
    }
}

fn load_history(state_dir: &Path, role_id: &str) -> RestartHistory {
    let path = restarts_path(state_dir, role_id);
    let raw = std::fs::read(&path).unwrap_or_default();
    serde_json::from_slice(&raw).unwrap_or_default()
}

fn save_history(state_dir: &Path, role_id: &str, history: &RestartHistory) -> io::Result<()> {
    let payload =
        serde_json::to_vec(history).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_atomic(&restarts_path(state_dir, role_id), &payload)
}

/// Publish one status envelope atomically, bumping the heartbeat counter.
fn publish(
    status_file: &Path,
    counter: &mut u64,
    observation: SupervisorObservation,
) -> io::Result<()> {
    *counter = counter.saturating_add(1);
    let envelope = ParentStatusEnvelope {
        schema: cynic_protocol::supervision::PARENT_STATUS_FILE_SCHEMA.to_string(),
        heartbeat_counter: *counter,
        observation,
    };
    let payload = serde_json::to_vec_pretty(&envelope)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_atomic(status_file, &payload)
}

/// Run the parent loop for one role until clean exit or give-up.
/// Returns the outcome; the child handle is held for the whole run.
pub async fn run_role(cfg: &RoleRunConfig) -> io::Result<RoleRunOutcome> {
    if cfg.command.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "role command must be non-empty",
        ));
    }
    let takeover = acquire_lock(&cfg.state_dir, &cfg.role_id)?;
    let mut history = load_history(&cfg.state_dir, &cfg.role_id);
    let mut counter: u64 = 0;
    // Heartbeat cadence is coupled to the `is_role_parented` freshness bound
    // (2x the default 5s interval): a slower heartbeat would read as unparented.
    let heartbeat_every = cfg.heartbeat_interval_secs.clamp(1, 5);
    #[cfg(unix)]
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
    #[cfg(unix)]
    let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).ok();
    let mut residuals: Vec<String> = takeover.into_iter().collect();
    let mut last_exit: Option<String> = None;

    // Expire history outside the policy window. The window covers a full
    // backoff sequence with margin (sum of capped delays is ~155s at
    // 5/60/5; 60x min_uptime gives 600s), so GiveUp holds across parent
    // restarts without remembering ancient failures forever.
    if now_ms().saturating_sub(history.window_start_ms)
        > (cfg.policy.min_uptime_secs as u128)
            .saturating_mul(1000)
            .saturating_mul(60)
    {
        history = RestartHistory::default();
    }
    if history.count == 0 {
        history.window_start_ms = now_ms();
    }

    loop {
        let attempt = history.count;
        let started = now_ms();
        let mut child = spawn_backend(&cfg.command, cfg.working_dir.as_deref(), &cfg.env)?;
        publish(
            &cfg.status_file,
            &mut counter,
            SupervisorObservation {
                schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA.to_string(),
                role_id: cfg.role_id.clone(),
                state: SupervisorState::Active,
                restart_count: attempt,
                last_exit_reason: last_exit.clone(),
                observed_at_unix_ms: now_ms(),
                residual_unknowns: residuals.clone(),
            },
        )?;

        // Wait for the child, heartbeating and watching for stop requests.
        // Poll-based (no `select!` borrow of the child): `try_wait` borrows
        // end immediately, so `graceful_stop` can take the child later.
        let mut elapsed_secs: u64 = 0;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            #[cfg(unix)]
            let signaled = wait_step(sigterm.as_mut(), sigint.as_mut()).await;
            #[cfg(not(unix))]
            let signaled = wait_step((), ()).await;
            if signaled {
                graceful_stop(&mut child, cfg.stop_grace_secs).await?;
                publish(
                    &cfg.status_file,
                    &mut counter,
                    SupervisorObservation {
                        schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA
                            .to_string(),
                        role_id: cfg.role_id.clone(),
                        state: SupervisorState::Stopped,
                        restart_count: attempt,
                        last_exit_reason: Some("signal_shutdown".to_string()),
                        observed_at_unix_ms: now_ms(),
                        residual_unknowns: Vec::new(),
                    },
                )?;
                release_lock(&cfg.state_dir, &cfg.role_id);
                return Ok(RoleRunOutcome::Clean);
            }
            elapsed_secs = elapsed_secs.saturating_add(1);
            if elapsed_secs % heartbeat_every == 0 {
                publish(
                    &cfg.status_file,
                    &mut counter,
                    SupervisorObservation {
                        schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA
                            .to_string(),
                        role_id: cfg.role_id.clone(),
                        state: SupervisorState::Active,
                        restart_count: attempt,
                        last_exit_reason: last_exit.clone(),
                        observed_at_unix_ms: now_ms(),
                        residual_unknowns: residuals.clone(),
                    },
                )?;
                touch_lock(&cfg.state_dir, &cfg.role_id);
            }
            if consume_stop_request(&cfg.state_dir, &cfg.role_id) {
                graceful_stop(&mut child, cfg.stop_grace_secs).await?;
                publish(
                    &cfg.status_file,
                    &mut counter,
                    SupervisorObservation {
                        schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA
                            .to_string(),
                        role_id: cfg.role_id.clone(),
                        state: SupervisorState::Stopped,
                        restart_count: attempt,
                        last_exit_reason: Some("stop_requested".to_string()),
                        observed_at_unix_ms: now_ms(),
                        residual_unknowns: Vec::new(),
                    },
                )?;
                release_lock(&cfg.state_dir, &cfg.role_id);
                return Ok(RoleRunOutcome::Clean);
            }
        };

        let uptime_secs = now_ms().saturating_sub(started) / 1000;
        let uptime_secs = uptime_secs.min(u64::MAX as u128) as u64;
        if status.success() {
            publish(
                &cfg.status_file,
                &mut counter,
                SupervisorObservation {
                    schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA.to_string(),
                    role_id: cfg.role_id.clone(),
                    state: SupervisorState::Stopped,
                    restart_count: attempt,
                    last_exit_reason: Some("clean_exit".to_string()),
                    observed_at_unix_ms: now_ms(),
                    residual_unknowns: residuals.clone(),
                },
            )?;
            release_lock(&cfg.state_dir, &cfg.role_id);
            return Ok(RoleRunOutcome::Clean);
        }

        last_exit = Some(format!("child_exit:{status}"));
        residuals.clear();
        match decide_restart(&cfg.policy, attempt, uptime_secs, true) {
            RestartDecision::Restart { delay_secs } => {
                history.count = history.count.saturating_add(1);
                save_history(&cfg.state_dir, &cfg.role_id, &history)?;
                publish(
                    &cfg.status_file,
                    &mut counter,
                    SupervisorObservation {
                        schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA
                            .to_string(),
                        role_id: cfg.role_id.clone(),
                        state: SupervisorState::Failed,
                        restart_count: history.count,
                        last_exit_reason: last_exit.clone(),
                        observed_at_unix_ms: now_ms(),
                        residual_unknowns: Vec::new(),
                    },
                )?;
                // Interruptible backoff wait: a stop request during the delay
                // stops without spawning again; a signal stops immediately.
                let mut waited: u64 = 0;
                let mut stopped_early = false;
                while waited < delay_secs {
                    #[cfg(unix)]
                    let signaled = wait_step(sigterm.as_mut(), sigint.as_mut()).await;
                    #[cfg(not(unix))]
                    let signaled = wait_step((), ()).await;
                    waited = waited.saturating_add(1);
                    if signaled {
                        last_exit = Some("signal_shutdown".to_string());
                    }
                    if signaled || consume_stop_request(&cfg.state_dir, &cfg.role_id) {
                        if !signaled {
                            last_exit = Some("stop_requested".to_string());
                        }
                        stopped_early = true;
                        break;
                    }
                }
                if stopped_early {
                    publish(
                        &cfg.status_file,
                        &mut counter,
                        SupervisorObservation {
                            schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA
                                .to_string(),
                            role_id: cfg.role_id.clone(),
                            state: SupervisorState::Stopped,
                            restart_count: history.count,
                            last_exit_reason: Some("stop_requested".to_string()),
                            observed_at_unix_ms: now_ms(),
                            residual_unknowns: Vec::new(),
                        },
                    )?;
                    release_lock(&cfg.state_dir, &cfg.role_id);
                    return Ok(RoleRunOutcome::Clean);
                }
            }
            RestartDecision::GiveUp { reason } => {
                publish(
                    &cfg.status_file,
                    &mut counter,
                    SupervisorObservation {
                        schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA
                            .to_string(),
                        role_id: cfg.role_id.clone(),
                        state: SupervisorState::Failed,
                        restart_count: history.count,
                        last_exit_reason: Some(reason.to_string()),
                        observed_at_unix_ms: now_ms(),
                        residual_unknowns: Vec::new(),
                    },
                )?;
                release_lock(&cfg.state_dir, &cfg.role_id);
                return Ok(RoleRunOutcome::GiveUp {
                    reason: reason.to_string(),
                });
            }
            RestartDecision::NoRestart => {
                release_lock(&cfg.state_dir, &cfg.role_id);
                return Ok(RoleRunOutcome::Clean);
            }
        }
    }
}

fn release_lock(state_dir: &Path, role_id: &str) {
    let _ = std::fs::remove_file(lock_path(state_dir, role_id));
}

/// Stop a running child with the shared ladder (re-exported for CLI stop arms later).
pub async fn stop_child(child: &mut Box<dyn ChildWrapper>, timeout_secs: u64) -> io::Result<()> {
    graceful_stop(child, timeout_secs).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::supervision::RestartPolicy;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cynic-parent-test-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn test_policy() -> RestartPolicy {
        RestartPolicy {
            max_attempts: 2,
            initial_delay_secs: 0,
            max_delay_secs: 0,
            min_uptime_secs: u64::MAX,
        }
    }

    #[test]
    fn is_role_parented_tracks_lock_lifetime() {
        let dir = test_dir("parented-roundtrip");
        assert!(!is_role_parented(&dir, "node"));
        acquire_lock(&dir, "node").unwrap();
        assert!(is_role_parented(&dir, "node"));
        assert!(!is_role_parented(&dir, "kernel"));
        release_lock(&dir, "node");
        assert!(!is_role_parented(&dir, "node"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn second_parent_refuses_while_first_holds_lock() {
        let dir = test_dir("lock-contention");
        let first = acquire_lock(&dir, "node");
        assert!(first.unwrap().is_none());
        let second = acquire_lock(&dir, "node");
        let err = second.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert!(err
            .to_string()
            .contains("supervisor_parent_holds_role:node"));
        release_lock(&dir, "node");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn failing_child_reaches_give_up_with_status_file() {
        let dir = test_dir("give-up");
        let status_file = dir.join("supervise-node.status.json");
        let cfg = RoleRunConfig {
            role_id: "node".to_string(),
            command: vec!["/bin/false".to_string()],
            working_dir: None,
            env: HashMap::new(),
            policy: test_policy(),
            stop_grace_secs: 1,
            heartbeat_interval_secs: 1,
            state_dir: dir.clone(),
            status_file: status_file.clone(),
        };
        let outcome = run_role(&cfg).await.unwrap();
        assert_eq!(
            outcome,
            RoleRunOutcome::GiveUp {
                reason: "max_restart_attempts_reached".to_string()
            }
        );
        let raw = std::fs::read(&status_file).unwrap();
        let envelope: ParentStatusEnvelope = serde_json::from_slice(&raw).unwrap();
        assert_eq!(envelope.observation.state, SupervisorState::Failed);
        assert_eq!(envelope.observation.restart_count, 2);
        assert_eq!(
            envelope.observation.last_exit_reason,
            Some("max_restart_attempts_reached".to_string())
        );
        assert!(envelope.heartbeat_counter >= 3);
        release_lock(&dir, "node");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn clean_child_exits_clean_without_restart() {
        let dir = test_dir("clean");
        let status_file = dir.join("supervise-node.status.json");
        let cfg = RoleRunConfig {
            role_id: "node".to_string(),
            command: vec!["/bin/true".to_string()],
            working_dir: None,
            env: HashMap::new(),
            policy: test_policy(),
            stop_grace_secs: 1,
            heartbeat_interval_secs: 1,
            state_dir: dir.clone(),
            status_file: status_file.clone(),
        };
        let outcome = run_role(&cfg).await.unwrap();
        assert_eq!(outcome, RoleRunOutcome::Clean);
        let raw = std::fs::read(&status_file).unwrap();
        let envelope: ParentStatusEnvelope = serde_json::from_slice(&raw).unwrap();
        assert_eq!(envelope.observation.state, SupervisorState::Stopped);
        assert_eq!(envelope.observation.restart_count, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod stop_drill_tests {
    use super::*;
    use cynic_protocol::supervision::{ParentCommand, RestartPolicy};

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cynic-stop-drill-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[tokio::test]
    async fn stop_request_stops_running_child_with_proof() {
        let dir = test_dir("running");
        let status_file = dir.join("supervise-node.status.json");
        let cfg = RoleRunConfig {
            role_id: "node".to_string(),
            command: vec!["/bin/sleep".to_string(), "60".to_string()],
            working_dir: None,
            env: HashMap::new(),
            policy: RestartPolicy {
                max_attempts: 5,
                initial_delay_secs: 0,
                max_delay_secs: 0,
                min_uptime_secs: u64::MAX,
            },
            stop_grace_secs: 2,
            heartbeat_interval_secs: 1,
            state_dir: dir.clone(),
            status_file: status_file.clone(),
        };
        let run = tokio::spawn(async move { run_role(&cfg).await });
        // Let the parent spawn and publish, then request stop.
        tokio::time::sleep(Duration::from_millis(1500)).await;
        let cmd = ParentCommand::stop(now_ms());
        let payload = serde_json::to_vec(&cmd).unwrap();
        write_atomic(&cmd_path(&dir, "node"), &payload).unwrap();
        let outcome = tokio::time::timeout(Duration::from_secs(15), run)
            .await
            .expect("parent should stop promptly")
            .expect("task join")
            .unwrap();
        assert_eq!(outcome, RoleRunOutcome::Clean);
        // Command file consumed exactly once.
        assert!(!cmd_path(&dir, "node").exists());
        // Status file proves the stop.
        let raw = std::fs::read(&status_file).unwrap();
        let envelope: ParentStatusEnvelope = serde_json::from_slice(&raw).unwrap();
        assert_eq!(envelope.observation.state, SupervisorState::Stopped);
        assert_eq!(
            envelope.observation.last_exit_reason,
            Some("stop_requested".to_string())
        );
        // Lock released: a new parent may take over.
        assert!(!lock_path(&dir, "node").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod signal_step_tests {
    use super::*;

    #[tokio::test]
    async fn wait_step_returns_false_without_signal() {
        #[cfg(unix)]
        let out = wait_step(None, None).await;
        #[cfg(not(unix))]
        let out = wait_step((), ()).await;
        assert!(!out);
    }

    #[test]
    fn consume_rejects_wrong_schema_and_unknown_command() {
        let dir = std::env::temp_dir().join("cynic-consume-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = cmd_path(&dir, "node");
        std::fs::write(
            &path,
            r#"{"schema":"bogus","command":"stop","requested_at_ms":0}"#,
        )
        .unwrap();
        assert!(!consume_stop_request(&dir, "node"));
        assert!(path.exists(), "unconsumed file must stay");
        std::fs::write(
            &path,
            r#"{"schema":"cynic.supervisor_parent_command.v1","command":"reboot","requested_at_ms":0}"#,
        )
        .unwrap();
        assert!(!consume_stop_request(&dir, "node"));
        assert!(path.exists(), "unknown command must stay");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
