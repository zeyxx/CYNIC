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

use cynic_protocol::supervision::{SUPERVISED_ROLE_IDS, SupervisorObservation};

#[derive(Subcommand, Debug, Clone)]
pub enum SuperviseCommand {
    /// Observe declared supervisor roles (read-only, JSON observations).
    Observe {
        /// Restrict output to one declared role id (`node`, `kernel`, `mcp`).
        #[arg(long)]
        role: Option<String>,
    },
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Build one explicit-unknown observation per declared role.
///
/// Pure constructor over the role list: no process table scan, no service
/// manager call, no shell. Live state arrives with the supervised parent.
pub(crate) fn observe_roles(role_filter: Option<&str>) -> Result<Vec<SupervisorObservation>, String> {
    if let Some(wanted) = role_filter {
        if !SUPERVISED_ROLE_IDS.contains(&wanted) {
            return Err(format!(
                "unknown role '{wanted}'; declared roles: {}",
                SUPERVISED_ROLE_IDS.join(", ")
            ));
        }
    }
    let now = now_unix_ms();
    Ok(SUPERVISED_ROLE_IDS
        .iter()
        .filter(|id| role_filter.is_none_or(|wanted| **id == wanted))
        .map(|id| {
            SupervisorObservation::unknown_role(
                id,
                now,
                &format!("supervisor_daemon_absent:{id}"),
            )
        })
        .collect())
}

pub async fn dispatch(cmd: SuperviseCommand) -> ExitCode {
    match cmd {
        SuperviseCommand::Observe { role } => match observe_roles(role.as_deref()) {
            Ok(observations) => crate::cli_json::emit_pretty_json(&observations),
            Err(message) => {
                eprintln!("{message}");
                ExitCode::FAILURE
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_all_roles_reports_explicit_unknown() {
        let observations = observe_roles(None).unwrap();
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
        assert!(observe_roles(Some("node")).unwrap().len() == 1);
        assert!(observe_roles(Some("nope")).is_err());
    }
}
