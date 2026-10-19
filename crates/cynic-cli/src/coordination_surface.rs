//! Fire-and-forget coordination surface: `cynic coordination append|read`.
//!
//! Thin projection over [`crate::coordination_log`]. Append never waits for
//! anyone; read converges by reading. Both resolve the log the same way:
//! explicit `--log`, else `CYNIC_COORDINATION_LOG`, else the operator-local
//! share dir.

use std::process::ExitCode;

use clap::Subcommand;

use crate::coordination_log::{
    CoordinationEntry, append_entry, default_coordination_log_path, read_entries,
};
use crate::emit_json;

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
    /// Project the unified coordination ledger.
    Ledger {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Explicit claims file path (else .agent-claims.json).
        #[arg(long)]
        claims: Option<String>,
        /// Explicit coordination log path (else CYNIC_COORDINATION_LOG, else local share).
        #[arg(long)]
        log: Option<String>,
        /// Explicit ledger path (else infra/coordination-ledger.json).
        #[arg(long)]
        ledger: Option<std::path::PathBuf>,
    },
    /// Project coordination metrics (active claims, response time, merge rate).
    Metrics {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Explicit claims file path (else .agent-claims.json).
        #[arg(long)]
        claims: Option<String>,
        /// Explicit coordination log path (else CYNIC_COORDINATION_LOG, else local share).
        #[arg(long)]
        log: Option<String>,
        /// Workspace root for git/PR data (else auto-detected).
        #[arg(long)]
        root: Option<String>,
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
        CoordinationCommand::Ledger {
            json,
            claims,
            log,
            ledger,
        } => {
            use crate::coordination_ledger::{COORDINATION_LEDGER_SCHEMA, CoordinationLedger};
            use crate::rust_source;

            let claims_path = claims.unwrap_or_else(|| {
                let root = rust_source::workspace_root();
                let p = root.join(".agent-claims.json");
                if p.exists() {
                    p.display().to_string()
                } else {
                    ".agent-claims.json".to_string()
                }
            });
            let log_path = match log.as_deref() {
                Some(p) if !p.trim().is_empty() => p.to_string(),
                _ => default_coordination_log_path()
                    .unwrap_or_else(|_| "coordination.jsonl".to_string()),
            };
            let ledger_path = ledger.unwrap_or_else(|| {
                rust_source::workspace_root().join("infra/coordination-ledger.json")
            });

            let ledger_doc = if ledger_path.exists() {
                let raw = std::fs::read_to_string(&ledger_path).unwrap_or_default();
                serde_json::from_str::<CoordinationLedger>(&raw)
                    .unwrap_or_else(|_| CoordinationLedger::new_unified(&claims_path, &log_path))
            } else {
                CoordinationLedger::new_unified(&claims_path, &log_path)
            };

            if json {
                emit_json(&ledger_doc, false);
            } else {
                println!("schema: {}", COORDINATION_LEDGER_SCHEMA);
                println!("status: {}", ledger_doc.status);
                println!(
                    "claims: {}",
                    ledger_doc.claims_path.clone().unwrap_or_default()
                );
                println!(
                    "coordination_log: {}",
                    ledger_doc.coordination_log_path.clone().unwrap_or_default()
                );
                for src in &ledger_doc.sources {
                    println!("source: {} {} {}", src.name, src.kind, src.path);
                }
            }
            ExitCode::SUCCESS
        }
        CoordinationCommand::Metrics {
            json,
            claims,
            log,
            root,
        } => {
            use crate::coordination_metrics::{
                COORDINATION_METRICS_SCHEMA, build_metrics, claims_from_document,
                count_active_claims, merge_rate_from_counts, response_time_from_entries,
            };
            use crate::rust_source;

            let ws_root = match root.as_deref() {
                Some(p) if !p.trim().is_empty() => std::path::PathBuf::from(p),
                _ => rust_source::workspace_root(),
            };

            let claims_path_str = claims.unwrap_or_else(|| {
                let p = ws_root.join(".agent-claims.json");
                if p.exists() {
                    p.display().to_string()
                } else {
                    ".agent-claims.json".to_string()
                }
            });
            let claims_doc: serde_json::Value = std::fs::read_to_string(&claims_path_str)
                .ok()
                .and_then(|raw| serde_json::from_str(&raw).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            let active_claims =
                count_active_claims(&claims_from_document(&claims_doc), now_unix_s());

            let log_path = match log.as_deref() {
                Some(p) if !p.trim().is_empty() => p.to_string(),
                _ => default_coordination_log_path()
                    .unwrap_or_else(|_| "coordination.jsonl".to_string()),
            };
            let (entries, _skipped) = read_entries(&log_path, None, 0);
            let response_time = response_time_from_entries(&entries);

            // Merge rate: prefer PR data via the sovereign VCS port, fall back to git.
            let mut merged = 0usize;
            let mut total = 0usize;
            let mut got_pr = false;
            if let Ok(port) = cynic_vcs_providers::forgejo_pr_port_from_registry(&ws_root) {
                if let Ok(prs) = port.list("all") {
                    total = prs.len();
                    merged = prs
                        .iter()
                        .filter(|p| format!("{:?}", p.state).eq_ignore_ascii_case("merged"))
                        .count();
                    got_pr = true;
                }
            }
            if !got_pr {
                // Fallback: git merge commits vs total commits on HEAD.
                let total_out = std::process::Command::new("git")
                    .args(["rev-list", "--count", "HEAD"])
                    .current_dir(&ws_root)
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .and_then(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .trim()
                            .parse::<usize>()
                            .ok()
                    })
                    .unwrap_or(0);
                let merged_out = std::process::Command::new("git")
                    .args(["rev-list", "--count", "--merges", "HEAD"])
                    .current_dir(&ws_root)
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .and_then(|o| {
                        String::from_utf8_lossy(&o.stdout)
                            .trim()
                            .parse::<usize>()
                            .ok()
                    })
                    .unwrap_or(0);
                total = total_out;
                merged = merged_out;
            }

            let merge_rate = merge_rate_from_counts(merged, total);

            let metrics = build_metrics(active_claims, response_time, merge_rate);

            if json {
                emit_json(&metrics, false);
            } else {
                println!("schema: {}", COORDINATION_METRICS_SCHEMA);
                println!("active_claims: {}", metrics.active_claims);
                println!(
                    "agent_response_time: samples={} min={:?} max={:?} avg={:?}",
                    metrics.agent_response_time.samples,
                    metrics.agent_response_time.min_seconds,
                    metrics.agent_response_time.max_seconds,
                    metrics.agent_response_time.avg_seconds
                );
                println!(
                    "merge_rate: {}/{} = {:.4}",
                    metrics.merge_rate.merged, metrics.merge_rate.total, metrics.merge_rate.rate
                );
            }
            ExitCode::SUCCESS
        }
    }
}
