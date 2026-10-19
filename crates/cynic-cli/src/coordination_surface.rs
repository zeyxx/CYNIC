//! Fire-and-forget coordination surface: `cynic coordination append|read`.
//!
//! Thin projection over [`crate::coordination_log`]. Append never waits for
//! anyone; read converges by reading. Both resolve the log the same way:
//! explicit `--log`, else `CYNIC_COORDINATION_LOG`, else the operator-local
//! share dir.

use std::process::ExitCode;

use clap::Subcommand;

use crate::emit_json;
use crate::coordination_log::{
    CoordinationEntry, append_entry, default_coordination_log_path, read_entries,
};

/// Agent coordination fire-and-forget log.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum CoordinationCommand {
    /// Append one entry. Returns without waiting for any reader.
    Append {
        /// Who appends (else CYNIC_AGENT_ID).
        #[arg(long, env = "CYNIC_AGENT_ID")]
        agent: Option<String>,
        /// Entry kind: debt, info, block, decision (free string, conventions).
        #[arg(long)]
        kind: String,
        /// Entry body (one matter, plain words).
        #[arg(long)]
        body: String,
        /// Reference anchors (PR numbers, commits, files). Repeatable.
        #[arg(long)]
        r#ref: Vec<String>,
        /// Explicit log path (else CYNIC_COORDINATION_LOG, else local share).
        #[arg(long)]
        log: Option<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Read entries, oldest first. Convergence by reading.
    Read {
        /// Only this kind.
        #[arg(long)]
        kind: Option<String>,
        /// Newest N entries (0 = all).
        #[arg(long, default_value = "0")]
        limit: usize,
        /// Explicit log path (else CYNIC_COORDINATION_LOG, else local share).
        #[arg(long)]
        log: Option<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

fn resolve_log(explicit: Option<&str>) -> Result<String, String> {
    match explicit {
        Some(path) if !path.trim().is_empty() => Ok(path.to_string()),
        _ => default_coordination_log_path(),
    }
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn execute_coordination(command: CoordinationCommand) -> ExitCode {
    match command {
        CoordinationCommand::Append {
            agent,
            kind,
            body,
            r#ref,
            log,
            json,
        } => {
            let agent = agent.unwrap_or_default();
            let entry = CoordinationEntry::new(&agent, &kind, &body, r#ref, now_unix_s());
            let log_path = match resolve_log(log.as_deref()) {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("coordination append failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            match append_entry(&log_path, &entry) {
                Ok(()) => {
                    if json {
                        emit_json(&entry, false);
                    } else {
                        println!("appended {} entry to {}", entry.kind, log_path);
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("coordination append failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        CoordinationCommand::Read {
            kind,
            limit,
            log,
            json,
        } => {
            let log_path = match resolve_log(log.as_deref()) {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("coordination read failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let (entries, skipped) = read_entries(&log_path, kind.as_deref(), limit);
            if json {
                emit_json(
                    &serde_json::json!({
                    "entries": entries,
                    "skipped_lines": skipped,
                    "log": log_path,
                }),
                    false,
                );
            } else {
                for entry in &entries {
                    let refs = if entry.refs.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", entry.refs.join(", "))
                    };
                    println!(
                        "{} {} [{}] {}{}",
                        entry.ts_unix_s, entry.agent, entry.kind, entry.body, refs
                    );
                }
                if skipped > 0 {
                    eprintln!("warning: {skipped} corrupt line(s) skipped");
                }
            }
            ExitCode::SUCCESS
        }
    }
}
