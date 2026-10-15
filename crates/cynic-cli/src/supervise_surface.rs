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
        /// Restrict output to one declared role id (`node`, `kernel`, `mcp`).
        #[arg(long)]
        role: Option<String>,
        /// Override the parent status file path (default per role under `/run/cynic`).
        #[arg(long)]
        status_file: Option<String>,
    },
    /// Run the supervised parent for one role (long-running, holds the child handle).
    Run {
        /// Declared role id to parent (`node`, `kernel`, `mcp`).
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
        /// Declared role argv, everything after `--`.
        #[arg(last = true)]
        command: Vec<String>,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        use cynic_protocol::supervision::{ParentStatusEnvelope, SupervisorState};
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
}
