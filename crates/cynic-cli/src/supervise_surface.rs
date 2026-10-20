//! Sovereign supervisor observation surface — `cynic supervise observe`.
//!
//! Read-only by construction: this module contains no spawn, no signal, and
//! no process-group code path. It reports declared roles from the protocol
//! materialisation of `contracts/supervisor-contract.json` and marks every
//! role `Unknown` with an explicit residual unknown until a supervisor
//! daemon holds live state (S3b/G1). Unknown is data, never a placeholder
//! for active.

use clap::Subcommand;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::supervision::{
    SupervisorObservation, PARENT_STATUS_STALE_AFTER_MS, SUPERVISED_ROLE_IDS,
};

#[derive(Subcommand, Debug, Clone)]
pub enum SuperviseCommand {
    /// Observe declared supervisor roles (read-only, JSON observations).
    Observe {
        /// Restrict output to one declared role id (`node`, `kernel`, `mcp`, `inference`).
        #[arg(long)]
        role: Option<String>,
        /// Override the parent status file path (default per role under `/run/cynic`).
        #[arg(long)]
        status_file: Option<String>,
    },
    /// Run the supervised parent for one role (long-running, holds the child handle).
    Run {
        /// Declared role id to parent (`node`, `kernel`, `mcp`, `inference`).
        #[arg(long)]
        role: String,
        /// State directory for the single-parent lock and restart history.
        #[arg(long, default_value = "/var/lib/cynic/supervise")]
        state_dir: String,
        /// Status file the parent publishes atomically for `observe`.
        #[arg(long)]
        status_file: Option<String>,
        /// Restart bound (contract declares none; node practice is 5).
        #[arg(long, default_value_t = 5)]
        max_attempts: u32,
        /// Contract `lifecycle.restart.backoff.initial_seconds`.
        #[arg(long, default_value_t = cynic_protocol::supervision::SUPERVISOR_BACKOFF_INITIAL_SECS)]
        initial_delay_secs: u64,
        /// Contract `lifecycle.restart.backoff.max_seconds`.
        #[arg(long, default_value_t = cynic_protocol::supervision::SUPERVISOR_BACKOFF_MAX_SECS)]
        max_delay_secs: u64,
        /// Contract `lifecycle.restart.backoff.min_uptime_seconds`.
        #[arg(long, default_value_t = cynic_protocol::supervision::SUPERVISOR_MIN_UPTIME_SECS)]
        min_uptime_secs: u64,
        /// Contract `lifecycle.signals.stop.grace_seconds`.
        #[arg(long, default_value_t = cynic_protocol::supervision::SUPERVISOR_STOP_GRACE_SECS)]
        stop_grace_secs: u64,
        /// Status heartbeat cadence (capped at 5 by the parent run loop).
        #[arg(long, default_value_t = 5)]
        heartbeat_interval_secs: u64,
        /// Declared role argv, everything after `--`.
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// Request a supervised parent to stop its child (bounded mutation).
    Stop {
        /// Declared role id to stop (`node`, `kernel`, `mcp`, `inference`).
        #[arg(long)]
        role: String,
        /// State directory holding the parent lock and command file.
        #[arg(long, default_value = "/var/lib/cynic/supervise")]
        state_dir: String,
        /// Status file to read back the stop proof.
        #[arg(long)]
        status_file: Option<String>,
        /// Seconds to wait for the Stopped read-back before failing.
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
        /// Explicit mutation consent: without it, the command only prints
        /// what it would do.
        #[arg(long)]
        apply: bool,
    },
    /// Watch one role's parent heartbeat and append ledger entries
    /// (long-running consumer; records only, never restarts).
    Watch {
        /// Declared role id to watch (`node`, `kernel`, `mcp`, `inference`).
        #[arg(long)]
        role: String,
        /// Status file the parent publishes.
        #[arg(long)]
        status_file: Option<String>,
        /// Ledger file receiving one JSON line per recorded verdict.
        #[arg(long, default_value = "/var/lib/cynic/supervise/watch-ledger.jsonl")]
        ledger_file: String,
        /// Seconds between status file polls.
        #[arg(long, default_value_t = 5)]
        poll_interval_secs: u64,
        /// Milliseconds past which a heartbeat reads stale (2x the 5s
        /// producer cadence cap).
        #[arg(long, default_value_t = 10_000)]
        stale_after_ms: u128,
        /// Seconds between repeated fresh confirmations (bounds volume).
        #[arg(long, default_value_t = 3_600)]
        confirm_interval_secs: u64,
    },
}

/// Default status file path for a role (Linux instantiation of the spec).
pub(crate) fn default_status_file(role_id: &str) -> String {
    format!("/run/cynic/supervise-{role_id}.status.json")
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Read one parent status file. Returns `None` when no parent publishes
/// (absent/unreadable file); stale files are reported, never trusted.
fn read_parent_status(
    status_file: &str,
) -> Option<cynic_protocol::supervision::ParentStatusEnvelope> {
    let raw = std::fs::read(status_file).ok()?;
    serde_json::from_slice(&raw).ok()
}

/// Build one observation per declared role.
///
/// Roles with a fresh parent status file report the parent's state;
/// every other role reports explicit `Unknown` (absent parent, stale file,
/// or unreadable envelope — each with its own residual). Pure constructor
/// over the role list plus status-file reads: no spawn, no signal.
pub(crate) fn observe_roles(
    role_filter: Option<&str>,
    status_file_override: Option<&str>,
) -> Result<Vec<SupervisorObservation>, String> {
    if let Some(wanted) = role_filter {
        if !SUPERVISED_ROLE_IDS.contains(&wanted) {
            return Err(format!(
                "unknown role '{wanted}'; declared roles: {}",
                SUPERVISED_ROLE_IDS.join(", ")
            ));
        }
    }
    // A status-file override names the file for exactly one role.
    if status_file_override.is_some() && role_filter.is_none() {
        return Err("--status-file requires --role <id>".to_string());
    }
    let now = now_unix_ms();
    Ok(SUPERVISED_ROLE_IDS
        .iter()
        .filter(|id| role_filter.is_none_or(|wanted| **id == wanted))
        .map(|id| {
            let path = status_file_override
                .map(str::to_string)
                .unwrap_or_else(|| default_status_file(id));
            match read_parent_status(&path) {
                Some(envelope) => {
                    let age_ms = now.saturating_sub(envelope.observation.observed_at_unix_ms);
                    if age_ms <= PARENT_STATUS_STALE_AFTER_MS {
                        envelope.observation
                    } else {
                        SupervisorObservation::unknown_role(id, now, "supervisor_status_stale")
                    }
                }
                None => SupervisorObservation::unknown_role(
                    id,
                    now,
                    &format!("supervisor_daemon_absent:{id}"),
                ),
            }
        })
        .collect())
}

/// Write a stop request for the parent to consume exactly once.
/// Fails fast with a named error when no live parent holds the role, so the
/// operator never waits a full timeout on nothing. Re-requesting overwrites
/// the single slot (stop is idempotent); the parent deletes on consume.
pub(crate) fn request_stop(state_dir: &str, role_id: &str) -> Result<String, String> {
    if !cynic_supervisor::is_role_parented(std::path::Path::new(state_dir), role_id) {
        return Err(format!(
            "no supervised parent holds role '{role_id}'; nothing to stop"
        ));
    }
    let path = std::path::PathBuf::from(state_dir).join(format!("supervise-{role_id}.cmd.json"));
    let cmd = cynic_protocol::supervision::ParentCommand::stop(now_unix_ms());
    let payload = serde_json::to_vec(&cmd).map_err(|e| format!("command encode failed: {e}"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("state dir failed: {e}"))?;
    }
    std::fs::write(&path, &payload).map_err(|e| format!("command write failed: {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Wait for the Stopped read-back proof, polling the status file.
/// Returns the final observation on proof, `None` on timeout.
pub(crate) async fn await_stopped(
    status_file: &str,
    timeout_secs: u64,
) -> Option<SupervisorObservation> {
    let deadline = now_unix_ms().saturating_add((timeout_secs as u128).saturating_mul(1000));
    loop {
        if let Some(envelope) = read_parent_status(status_file) {
            if envelope.observation.state == cynic_protocol::supervision::SupervisorState::Stopped {
                return Some(envelope.observation);
            }
        }
        if now_unix_ms() >= deadline {
            return None;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

pub async fn dispatch(cmd: SuperviseCommand) -> ExitCode {
    match cmd {
        SuperviseCommand::Observe { role, status_file } => {
            match observe_roles(role.as_deref(), status_file.as_deref()) {
                Ok(observations) => crate::cli_json::emit_pretty_json(&observations),
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::FAILURE
                }
            }
        }
        SuperviseCommand::Run {
            role,
            state_dir,
            status_file,
            max_attempts,
            initial_delay_secs,
            max_delay_secs,
            min_uptime_secs,
            stop_grace_secs,
            heartbeat_interval_secs,
            command,
        } => {
            if !SUPERVISED_ROLE_IDS.contains(&role.as_str()) {
                eprintln!(
                    "unknown role '{role}'; declared roles: {}",
                    SUPERVISED_ROLE_IDS.join(", ")
                );
                return ExitCode::FAILURE;
            }
            if command.is_empty() {
                eprintln!("role command must be non-empty; pass the declared argv after `--`");
                return ExitCode::FAILURE;
            }
            let cfg = cynic_supervisor::RoleRunConfig {
                role_id: role.clone(),
                command,
                working_dir: None,
                env: std::collections::HashMap::new(),
                policy: cynic_protocol::supervision::RestartPolicy {
                    max_attempts,
                    initial_delay_secs,
                    max_delay_secs,
                    min_uptime_secs,
                },
                stop_grace_secs,
                heartbeat_interval_secs,
                state_dir: std::path::PathBuf::from(state_dir),
                status_file: std::path::PathBuf::from(
                    status_file.unwrap_or_else(|| default_status_file(&role)),
                ),
            };
            match cynic_supervisor::run_role(&cfg).await {
                Ok(cynic_supervisor::RoleRunOutcome::Clean) => ExitCode::SUCCESS,
                Ok(cynic_supervisor::RoleRunOutcome::GiveUp { reason }) => {
                    eprintln!("role '{role}' gave up: {reason}");
                    ExitCode::FAILURE
                }
                Err(error) => {
                    eprintln!("supervise run failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        SuperviseCommand::Stop {
            role,
            state_dir,
            status_file,
            timeout_secs,
            apply,
        } => {
            if !SUPERVISED_ROLE_IDS.contains(&role.as_str()) {
                eprintln!(
                    "unknown role '{role}'; declared roles: {}",
                    SUPERVISED_ROLE_IDS.join(", ")
                );
                return ExitCode::FAILURE;
            }
            let status_path = status_file.unwrap_or_else(|| default_status_file(&role));
            if !apply {
                println!(
                    "would request stop for role '{role}' (command file under {state_dir}); re-run with --apply"
                );
                return ExitCode::SUCCESS;
            }
            match request_stop(&state_dir, &role) {
                Ok(_) => match await_stopped(&status_path, timeout_secs).await {
                    Some(observation) => crate::cli_json::emit_pretty_json(&observation),
                    None => {
                        eprintln!(
                            "stop requested for role '{role}' but no Stopped read-back within {timeout_secs}s"
                        );
                        ExitCode::FAILURE
                    }
                },
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::FAILURE
                }
            }
        }
        SuperviseCommand::Watch {
            role,
            status_file,
            ledger_file,
            poll_interval_secs,
            stale_after_ms,
            confirm_interval_secs,
        } => {
            if !SUPERVISED_ROLE_IDS.contains(&role.as_str()) {
                eprintln!(
                    "unknown role '{role}'; declared roles: {}",
                    SUPERVISED_ROLE_IDS.join(", ")
                );
                return ExitCode::FAILURE;
            }
            let cfg = cynic_supervisor::WatchConfig {
                role_id: role.clone(),
                status_file: std::path::PathBuf::from(
                    status_file.unwrap_or_else(|| default_status_file(&role)),
                ),
                ledger_file: std::path::PathBuf::from(ledger_file),
                poll_interval_secs,
                stale_after_ms,
                confirm_interval_secs,
            };
            match cynic_supervisor::watch_role(&cfg).await {
                Ok(cynic_supervisor::WatchRunOutcome::Signaled) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("supervise watch failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::supervision::{ParentStatusEnvelope, SupervisorState};

    #[test]
    fn observe_all_roles_reports_explicit_unknown() {
        let observations = observe_roles(None, None).unwrap();
        assert_eq!(observations.len(), SUPERVISED_ROLE_IDS.len());
        for obs in &observations {
            assert_eq!(
                obs.state,
                cynic_protocol::supervision::SupervisorState::Unknown
            );
            assert!(!obs.residual_unknowns.is_empty());
        }
    }

    #[test]
    fn observe_role_filter_rejects_undeclared() {
        assert!(observe_roles(Some("node"), None).unwrap().len() == 1);
        assert!(observe_roles(Some("nope"), None).is_err());
        assert!(observe_roles(None, Some("/tmp/x.json")).is_err());
    }

    #[test]
    fn observe_reads_fresh_parent_status_file() {
        let dir = std::env::temp_dir().join("cynic-observe-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("node.status.json");
        let obs = SupervisorObservation {
            schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA.to_string(),
            role_id: "node".to_string(),
            state: SupervisorState::Active,
            restart_count: 3,
            last_exit_reason: None,
            observed_at_unix_ms: now_unix_ms(),
            residual_unknowns: Vec::new(),
        };
        let envelope = ParentStatusEnvelope::new(obs);
        std::fs::write(&path, serde_json::to_vec(&envelope).unwrap()).unwrap();
        let observations = observe_roles(Some("node"), Some(path.to_str().unwrap())).unwrap();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].state, SupervisorState::Active);
        assert_eq!(observations[0].restart_count, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stop_request_needs_live_parent_and_rewrites_idempotently() {
        // No parent: fail fast, no file written.
        let dir = std::env::temp_dir().join("cynic-stop-req-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let err = request_stop(dir.to_str().unwrap(), "node").unwrap_err();
        assert!(
            err.contains("no supervised parent holds role"),
            "got: {err}"
        );
        // Live parent lock: request written, re-request overwrites the slot.
        let lock = format!(
            r#"{{"pid":{},"epoch":"test-epoch","started_at_ms":0}}"#,
            std::process::id()
        );
        std::fs::write(dir.join("supervise-node.lock"), lock).unwrap();
        let first = request_stop(dir.to_str().unwrap(), "node").unwrap();
        let second = request_stop(dir.to_str().unwrap(), "node").unwrap();
        assert_eq!(first, second);
        assert!(std::path::Path::new(&first).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn await_stopped_proves_from_status_file() {
        let dir = std::env::temp_dir().join("cynic-await-stop-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("node.status.json");
        // Missing file + zero timeout: no proof.
        assert!(await_stopped(path.to_str().unwrap(), 0).await.is_none());
        // A Stopped envelope is immediate proof.
        let obs = SupervisorObservation {
            schema: cynic_protocol::supervision::SUPERVISOR_OBSERVATION_SCHEMA.to_string(),
            role_id: "node".to_string(),
            state: SupervisorState::Stopped,
            restart_count: 1,
            last_exit_reason: Some("stop_requested".to_string()),
            observed_at_unix_ms: now_unix_ms(),
            residual_unknowns: Vec::new(),
        };
        std::fs::write(
            &path,
            serde_json::to_vec(&ParentStatusEnvelope::new(obs)).unwrap(),
        )
        .unwrap();
        let proof = await_stopped(path.to_str().unwrap(), 5).await.unwrap();
        assert_eq!(proof.state, SupervisorState::Stopped);
        assert_eq!(proof.last_exit_reason, Some("stop_requested".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
