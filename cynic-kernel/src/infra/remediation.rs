//! SSH Remediation Executor — reads TOML config, tracks recovery attempts with
//! bounded retries + cooldown, and executes restart commands via SSH.
//!
//! Called by the remediation watcher (Task 4) via `spawn_blocking` because
//! `ssh_restart` uses `std::process::Command` (blocking I/O).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ── defaults ─────────────────────────────────────────────────────────────────

fn default_max_retries() -> u32 {
    3
}

fn default_cooldown_secs() -> u64 {
    60
}

// ── config types ─────────────────────────────────────────────────────────────

/// Per-dog remediation configuration loaded from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct DogRemediation {
    /// SSH target, e.g. `"user@<TAILSCALE_NODE>"`
    pub node: String,
    /// Health check URL, e.g. `"http://<TAILSCALE_NODE>:8080/health"`
    pub health_url: String,
    /// Command to execute on the remote node to restart the service
    pub restart_command: String,
    /// Maximum recovery attempts before giving up (default: 3)
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// Minimum seconds between restart attempts (default: 60)
    #[serde(default = "default_cooldown_secs")]
    pub cooldown_secs: u64,
}

/// Wrapper that maps the `[dog.NAME]` TOML structure.
#[derive(Debug, Deserialize)]
struct RemediationFile {
    #[serde(default)]
    dog: HashMap<String, DogRemediation>,
}

// ── loader ────────────────────────────────────────────────────────────────────

/// Load remediation config from a TOML file.
///
/// Returns an empty map (with a `klog!` warning) if the file is missing
/// or cannot be parsed — callers should treat the empty map as "no remediation
/// configured" rather than a fatal error.
pub fn load_remediation(path: &Path) -> HashMap<String, DogRemediation> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            klog!(
                "[Remediation] Config not found or unreadable at {}: {e}",
                path.display()
            );
            return HashMap::new();
        }
    };

    match toml::from_str::<RemediationFile>(&raw) {
        Ok(f) => f.dog,
        Err(e) => {
            klog!(
                "[Remediation] Failed to parse config at {}: {e}",
                path.display()
            );
            HashMap::new()
        }
    }
}

// ── recovery tracker ─────────────────────────────────────────────────────────

/// Per-dog recovery state kept inside the tracker's Mutex.
#[derive(Debug)]
struct DogState {
    attempts: u32,
    exhausted: bool,
    last_attempt: Option<Instant>,
}

impl DogState {
    fn fresh() -> Self {
        Self {
            attempts: 0,
            exhausted: false,
            last_attempt: None,
        }
    }
}

/// Thread-safe tracker for bounded recovery attempts with cooldown enforcement.
///
/// Uses `std::sync::Mutex` (not tokio) because `ssh_restart` is called from
/// `spawn_blocking` — sync context.
#[derive(Debug)]
pub struct RecoveryTracker {
    inner: Mutex<HashMap<String, DogState>>,
}

impl RecoveryTracker {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Returns `true` when:
    ///   1. Retries are not exhausted for this dog, AND
    ///   2. Either no previous attempt has been made OR the cooldown has elapsed.
    pub fn should_restart(&self, dog_id: &str, config: &super::config::BackendRemediation) -> bool {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = map.get(dog_id) else {
            return true; // never attempted — allow
        };

        if state.exhausted {
            return false;
        }

        // Enforce cooldown
        if state
            .last_attempt
            .is_some_and(|last| last.elapsed() < Duration::from_secs(config.cooldown_secs))
        {
            return false;
        }

        true
    }

    /// Record a restart attempt. Marks exhausted when `attempts >= max_retries`.
    pub fn record_attempt(&self, dog_id: &str, max_retries: u32) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let state = map
            .entry(dog_id.to_string())
            .or_insert_with(DogState::fresh);
        state.attempts += 1;
        state.last_attempt = Some(Instant::now());
        if state.attempts >= max_retries {
            state.exhausted = true;
            klog!(
                "[Remediation] Dog '{}': max retries ({}) reached — giving up",
                dog_id,
                max_retries
            );
        }
    }

    /// Reset tracker state for a dog (call when it recovers successfully).
    pub fn reset(&self, dog_id: &str) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(dog_id);
    }
}

impl Default for RecoveryTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ── SSH executor ──────────────────────────────────────────────────────────────

/// Execute a remote command via SSH using BatchMode (no interactive prompts).
///
/// Uses `std::process::Command` — must be called from a blocking context
/// (e.g., `tokio::task::spawn_blocking`).
///
/// Returns `Ok(stdout)` on success (exit 0), `Err(stderr)` otherwise.
pub fn ssh_restart(node: &str, command: &str) -> Result<String, String> {
    let output = Command::new("ssh")
        .args([
            "-o",
            "ConnectTimeout=5",
            "-o",
            "ServerAliveInterval=5",
            "-o",
            "ServerAliveCountMax=2",
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=accept-new", // Acceptable in Tailscale mesh (nodes auth'd by WireGuard)
            node,
            command,
        ])
        .output()
        .map_err(|e| format!("Failed to spawn ssh: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use tempfile::NamedTempFile;

    fn make_config(
        max_retries: u32,
        cooldown_secs: u64,
    ) -> super::super::config::BackendRemediation {
        super::super::config::BackendRemediation {
            node: "user@test-node".to_string(),
            restart_command: "systemctl restart foo".to_string(),
            max_retries,
            cooldown_secs,
        }
    }

    #[test]
    fn load_valid_toml() {
        let mut f = NamedTempFile::new().unwrap();
        // Use non-routable test IPs (192.0.2.x = TEST-NET per RFC 5737)
        write!(
            f,
            r#"
[dog.sovereign-stanislaz]
node = "user@192.0.2.1"
health_url = "http://192.0.2.1:8080/health"
restart_command = "schtasks /run /tn CynicSovereign"

[dog.sovereign-ubuntu]
node = "user@192.0.2.2"
health_url = "http://192.0.2.2:8080/health"
restart_command = "systemctl restart llama"
max_retries = 5
cooldown_secs = 120
"#
        )
        .unwrap();

        let map = load_remediation(f.path());
        assert_eq!(map.len(), 2);

        let stanislaz = map
            .get("sovereign-stanislaz")
            .expect("missing sovereign-stanislaz");
        assert_eq!(stanislaz.node, "user@192.0.2.1");
        assert_eq!(stanislaz.max_retries, 3, "should use default");
        assert_eq!(stanislaz.cooldown_secs, 60, "should use default");

        let ubuntu = map
            .get("sovereign-ubuntu")
            .expect("missing sovereign-ubuntu");
        assert_eq!(ubuntu.max_retries, 5);
        assert_eq!(ubuntu.cooldown_secs, 120);
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let map = load_remediation(Path::new(
            "/tmp/this-file-does-not-exist-cynic-remediation.toml",
        ));
        assert!(map.is_empty());
    }

    #[test]
    fn recovery_tracker_respects_cooldown() {
        let tracker = RecoveryTracker::new();
        // Large cooldown: 3600 seconds
        let config = make_config(5, 3600);

        // First attempt: no state recorded yet → allowed
        assert!(tracker.should_restart("dog-a", &config));

        // Record an attempt
        tracker.record_attempt("dog-a", config.max_retries);

        // Second check: cooldown not elapsed → blocked
        assert!(!tracker.should_restart("dog-a", &config));
    }

    #[test]
    fn recovery_tracker_exhausts_after_max_retries() {
        let tracker = RecoveryTracker::new();
        // Zero cooldown so we can record attempts without waiting
        let config = make_config(3, 0);

        for _ in 0..config.max_retries {
            assert!(tracker.should_restart("dog-b", &config));
            tracker.record_attempt("dog-b", config.max_retries);
        }

        // After max_retries attempts, should_restart must return false
        assert!(!tracker.should_restart("dog-b", &config));
    }

    #[test]
    fn recovery_tracker_reset_clears_state() {
        let tracker = RecoveryTracker::new();
        let config = make_config(1, 3600);

        // Exhaust retries
        tracker.record_attempt("dog-c", config.max_retries);
        assert!(!tracker.should_restart("dog-c", &config));

        // Reset — state is gone, fresh start allowed
        tracker.reset("dog-c");
        assert!(tracker.should_restart("dog-c", &config));
    }
}
