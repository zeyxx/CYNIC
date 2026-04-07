// Supervise concern: spawn, stop, backoff

// WHY: Backoff fields + spawn helpers consumed by the main watch loop (Task 5).
// Modules are empty stubs until Tasks 3-5 wire up the full lifecycle.
#![allow(dead_code)]

use std::{
    collections::HashMap,
    ffi::OsStr,
    io,
    time::{Duration, Instant},
};

use process_wrap::tokio::{ChildWrapper, CommandWrap};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use crate::config::RestartConfig;

// ── Backoff ───────────────────────────────────────────────────────────────────

/// Exponential backoff with a min-uptime stability reset.
///
/// Call `record_start()` after each spawn, then `reset_if_stable()` after the
/// process exits to reset the counter if the process lived long enough.
#[derive(Debug)]
pub(crate) struct Backoff {
    pub(crate) attempt: u32,
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
    min_uptime: Duration,
    last_start: Instant,
}

impl Backoff {
    /// Create a new Backoff with explicit parameters.
    pub(crate) fn new(
        max_attempts: u32,
        initial_delay: Duration,
        max_delay: Duration,
        min_uptime: Duration,
    ) -> Self {
        Self {
            attempt: 0,
            max_attempts,
            initial_delay,
            max_delay,
            min_uptime,
            // Safety: Instant::now() is always valid; initialized to "now"
            // so that the first reset_if_stable call sees elapsed ≥ min_uptime.
            last_start: Instant::now(),
        }
    }

    /// Build from a [`RestartConfig`] TOML section.
    pub(crate) fn from_config(cfg: &RestartConfig) -> Self {
        Self::new(
            cfg.max_attempts,
            Duration::from_secs(cfg.initial_delay_secs),
            Duration::from_secs(cfg.max_delay_secs),
            Duration::from_secs(cfg.min_uptime_secs),
        )
    }

    /// Record that the backend just started. Call this immediately after spawn.
    pub(crate) fn record_start(&mut self) {
        self.last_start = Instant::now();
    }

    /// If the process lived at least `min_uptime`, reset the attempt counter.
    pub(crate) fn reset_if_stable(&mut self) {
        if self.last_start.elapsed() >= self.min_uptime {
            self.attempt = 0;
        }
    }

    /// Unconditionally reset the attempt counter.
    pub(crate) fn reset(&mut self) {
        self.attempt = 0;
    }

    /// Returns `true` when `attempt >= max_attempts`.
    pub(crate) fn exhausted(&self) -> bool {
        self.attempt >= self.max_attempts
    }

    /// Increment the attempt counter and return the next delay (capped at `max_delay`).
    ///
    /// Delay formula: `initial_delay * 2^(attempt - 1)` after increment.
    pub(crate) fn next_delay(&mut self) -> Duration {
        self.attempt = self.attempt.saturating_add(1);
        // 2^(attempt-1) — attempt is at least 1 after the increment above.
        // Clamp the shift to prevent u64 overflow: 2^63 already exceeds any
        // realistic delay, and Duration::saturating_mul handles overflow anyway.
        let shift = (self.attempt - 1).min(30); // 2^30 ≈ 1 billion seconds, well past any cap
        let multiplier = 1u32.checked_shl(shift).unwrap_or(u32::MAX);
        let delay = self.initial_delay.saturating_mul(multiplier);
        delay.min(self.max_delay)
    }

    /// Sleep for `next_delay()`, but cancel immediately if `shutdown` is triggered.
    pub(crate) async fn wait_or_shutdown(&mut self, shutdown: &CancellationToken) {
        let delay = self.next_delay();
        tokio::select! {
            _ = sleep(delay) => {}
            _ = shutdown.cancelled() => {}
        }
    }
}

// ── spawn_backend ─────────────────────────────────────────────────────────────

/// Spawn the inference backend process.
///
/// Uses `process-wrap` for cross-platform orphan safety:
/// - Unix: `ProcessGroup::leader()` + `KillOnDrop`
/// - Windows: `JobObject` + `KillOnDrop`
///
/// Stdio is inherited (NOT piped) to avoid pipe-buffer deadlock.
/// Returns `Err` if `command` is empty.
pub(crate) fn spawn_backend(
    command: &[impl AsRef<OsStr>],
    working_dir: Option<&str>,
    env: &HashMap<String, String>,
) -> io::Result<Box<dyn ChildWrapper>> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "command must be non-empty"))?;

    let mut cmd = CommandWrap::with_new(program, |c| {
        c.args(args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        if let Some(dir) = working_dir {
            c.current_dir(dir);
        }
        for (k, v) in env {
            c.env(k, v);
        }
    });

    #[cfg(unix)]
    {
        use process_wrap::tokio::{KillOnDrop, ProcessGroup};
        cmd.wrap(ProcessGroup::leader()).wrap(KillOnDrop);
    }
    #[cfg(windows)]
    {
        use process_wrap::tokio::{JobObject, KillOnDrop};
        cmd.wrap(JobObject).wrap(KillOnDrop);
    }

    cmd.spawn()
}

// ── graceful_stop ─────────────────────────────────────────────────────────────

/// Gracefully stop a child process.
///
/// Sends SIGTERM (Unix) or initiates a kill (Windows), then waits up to
/// `timeout_secs`. If the process has not exited by then, sends SIGKILL and
/// waits unconditionally.
pub(crate) async fn graceful_stop(
    child: &mut Box<dyn ChildWrapper>,
    timeout_secs: u64,
) -> io::Result<()> {
    #[cfg(unix)]
    {
        // SIGTERM the process group (ProcessGroup wrapper propagates to group).
        // Ignore send errors — process may already be dead.
        let _ = child.signal(libc::SIGTERM);
    }
    #[cfg(not(unix))]
    {
        // Windows: initiate kill, then give it a moment to flush.
        let _ = child.start_kill();
    }

    let timeout = Duration::from_secs(timeout_secs);
    match tokio::time::timeout(timeout, child.wait()).await {
        Ok(result) => result.map(|_| ()),
        Err(_elapsed) => {
            // Timed out — force kill.
            let _ = child.start_kill();
            child.wait().await.map(|_| ())
        }
    }
}

// ── spawn_with_retries ────────────────────────────────────────────────────────

/// Try to spawn the backend up to `max` times with 1-second delays between
/// attempts. Returns the live child or the last error.
pub(crate) async fn spawn_with_retries(
    command: &[impl AsRef<OsStr>],
    working_dir: Option<&str>,
    env: &HashMap<String, String>,
    max: u32,
) -> io::Result<Box<dyn ChildWrapper>> {
    let mut last_err = io::Error::other("no attempts made");
    for _ in 0..max {
        match spawn_backend(command, working_dir, env) {
            Ok(child) => return Ok(child),
            Err(e) => {
                last_err = e;
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    Err(last_err)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Backoff tests ─────────────────────────────────────────────────────────

    #[test]
    fn backoff_progression() {
        // initial = 2s, max = 64s, max_attempts = 5
        let mut b = Backoff::new(
            5,
            Duration::from_secs(2),
            Duration::from_secs(64),
            Duration::from_secs(10),
        );
        // Attempt 1: 2 * 2^0 = 2s
        assert_eq!(b.next_delay(), Duration::from_secs(2));
        // Attempt 2: 2 * 2^1 = 4s
        assert_eq!(b.next_delay(), Duration::from_secs(4));
        // Attempt 3: 2 * 2^2 = 8s
        assert_eq!(b.next_delay(), Duration::from_secs(8));
        // Attempt 4: 2 * 2^3 = 16s
        assert_eq!(b.next_delay(), Duration::from_secs(16));
        // Attempt 5: 2 * 2^4 = 32s
        assert_eq!(b.next_delay(), Duration::from_secs(32));
        assert_eq!(b.attempt, 5);
    }

    #[test]
    fn backoff_caps_at_max() {
        let mut b = Backoff::new(
            10,
            Duration::from_secs(2),
            Duration::from_secs(30),
            Duration::from_secs(10),
        );
        // Drive past the cap
        for _ in 0..10 {
            let delay = b.next_delay();
            assert!(
                delay <= Duration::from_secs(30),
                "delay {delay:?} exceeded max"
            );
        }
        // The last delay must be exactly at the cap
        let mut b2 = Backoff::new(
            10,
            Duration::from_secs(2),
            Duration::from_secs(30),
            Duration::from_secs(10),
        );
        // 2*2^4=32 > 30, so attempt 5 should be capped at 30
        for _ in 0..4 {
            b2.next_delay();
        }
        assert_eq!(b2.next_delay(), Duration::from_secs(30));
    }

    #[test]
    fn backoff_reset_if_stable() {
        let mut b = Backoff::new(
            5,
            Duration::from_secs(2),
            Duration::from_secs(64),
            // min_uptime = 0 so last_start.elapsed() always >= 0
            Duration::from_secs(0),
        );
        b.next_delay();
        b.next_delay();
        assert_eq!(b.attempt, 2);
        b.record_start();
        // min_uptime = 0 → always stable
        b.reset_if_stable();
        assert_eq!(b.attempt, 0);
    }

    #[test]
    fn backoff_reset_if_stable_not_yet() {
        let mut b = Backoff::new(
            5,
            Duration::from_secs(2),
            Duration::from_secs(64),
            // min_uptime = 1 hour → process cannot have lived that long
            Duration::from_secs(3600),
        );
        b.next_delay();
        b.next_delay();
        assert_eq!(b.attempt, 2);
        b.record_start();
        // process "just started" — nowhere near min_uptime
        b.reset_if_stable();
        assert_eq!(b.attempt, 2, "should NOT reset — not stable yet");
    }

    #[test]
    fn backoff_reset_unconditional() {
        let mut b = Backoff::new(
            5,
            Duration::from_secs(2),
            Duration::from_secs(64),
            Duration::from_secs(3600),
        );
        b.next_delay();
        b.next_delay();
        b.next_delay();
        assert_eq!(b.attempt, 3);
        b.reset();
        assert_eq!(b.attempt, 0);
    }

    // ── spawn_backend / graceful_stop tests ───────────────────────────────────

    #[tokio::test]
    async fn spawn_and_wait() {
        let env: HashMap<String, String> = HashMap::new();
        let mut child = spawn_backend(&["echo", "hello"], None, &env).unwrap();
        let status = child.wait().await.unwrap();
        assert!(status.success(), "echo should exit 0, got {status}");
    }

    #[tokio::test]
    async fn graceful_stop_clean() {
        let env: HashMap<String, String> = HashMap::new();
        let mut child = spawn_backend(&["sleep", "60"], None, &env).unwrap();
        // graceful_stop with 5s timeout — should SIGTERM immediately
        graceful_stop(&mut child, 5).await.unwrap();
        // After graceful_stop the process must be dead (try_wait returns Some)
        let exited = child.try_wait().unwrap();
        assert!(
            exited.is_some(),
            "process should have exited after graceful_stop"
        );
    }

    #[tokio::test]
    async fn spawn_with_retries_bad_binary() {
        let env: HashMap<String, String> = HashMap::new();
        let result = spawn_with_retries(&["/nonexistent/binary"], None, &env, 3).await;
        assert!(result.is_err(), "bad binary should fail after retries");
    }
}
