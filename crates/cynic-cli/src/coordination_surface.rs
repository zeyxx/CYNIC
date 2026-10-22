//! Fire-and-forget coordination surface: `cynic coordination append|read`
//! plus addressed messaging (`send|inbox|ack`) on the same log.
//!
//! Thin projection over [`crate::coordination_log`]. Append never waits for
//! anyone; read converges by reading. Both resolve the log the same way:
//! explicit `--log`, else `CYNIC_COORDINATION_LOG`, else the operator-local
//! share dir.

use std::process::ExitCode;

use clap::Subcommand;

use crate::coordination_log::{
    CoordinationEntry, MsgReceipt, ReceiptState, admit_addressed_append, admit_receipt,
    append_entry, default_coordination_log_path, read_entries,
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
    /// Send an addressed message. Prints the msg_id proof, never waits.
    Send {
        /// Addressee: agent:<id> or role:<role>.
        #[arg(long)]
        to: String,
        /// Entry kind (free string, conventions).
        #[arg(long, default_value = "info")]
        kind: String,
        /// Triage summary, max 140 chars. Shown without opening the body.
        #[arg(long)]
        summary: String,
        /// Full body.
        #[arg(long)]
        body: String,
        /// Triage priority 0-3 (default 1).
        #[arg(long, default_value = "1")]
        priority: u8,
        /// Answer to this msg_id (causal thread).
        #[arg(long)]
        in_reply_to: Option<String>,
        /// Reference anchors. Repeatable.
        #[arg(long)]
        r#ref: Vec<String>,
        /// Who sends (else CYNIC_AGENT_ID).
        #[arg(long, env = "CYNIC_AGENT_ID")]
        agent: Option<String>,
        /// Explicit log path (else CYNIC_COORDINATION_LOG, else local share).
        #[arg(long)]
        log: Option<String>,
        /// Relay the entry to another node over SSH (layer 0; full peer tx later).
        /// Value is a raw SSH target (user@host or config alias).
        #[arg(long)]
        via: Option<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Inbox: addressed-to-me plus broadcast, summary-first. Body only with --full.
    Inbox {
        /// My agent id (else CYNIC_AGENT_ID).
        #[arg(long, env = "CYNIC_AGENT_ID")]
        agent: Option<String>,
        /// Only messages with no opened receipt.
        #[arg(long)]
        unopened: bool,
        /// Include full bodies, not just summaries.
        #[arg(long)]
        full: bool,
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
    /// Acknowledge a message: appends the receipt line. Monotonic only.
    Ack {
        /// The msg_id to acknowledge.
        msg_id: String,
        /// Receipt state: delivered or opened.
        #[arg(long, default_value = "delivered")]
        state: String,
        /// Who acknowledges (else CYNIC_AGENT_ID).
        #[arg(long, env = "CYNIC_AGENT_ID")]
        agent: Option<String>,
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

/// Single-quote a string for remote shell use. Closed over the whole word,
/// so bodies with spaces, quotes, or `$()` travel intact.
fn shell_quote(raw: &str) -> String {
    format!("'{}'", raw.replace('\'', "'\\''"))
}

/// Remote argv for one relayed entry. The original msg_id travels in `--ref`,
/// so the remote copy links back to the local proof even though the remote
/// log mints its own id. Pure: no I/O, fully unit-tested.
fn relay_argv(
    target: &str,
    sender: &str,
    kind: &str,
    body: &str,
    msg_id: &str,
    refs: &[String],
) -> Vec<String> {
    let mut argv = vec![
        "ssh".to_string(),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        target.to_string(),
        "cynic".to_string(),
        "coordination".to_string(),
        "append".to_string(),
        "--agent".to_string(),
        sender.to_string(),
        "--kind".to_string(),
        kind.to_string(),
        "--body".to_string(),
        shell_quote(body),
    ];
    for r in std::iter::once(&msg_id.to_string()).chain(refs.iter()) {
        argv.push("--ref".to_string());
        argv.push(shell_quote(r));
    }
    argv.push("--json".to_string());
    argv
}

/// Copy one entry into the destination node's coordination log over SSH.
/// Layer 0 of phase 2: the mesh ssh carries what peer tx will carry later.
/// Returns the remote receipt line on success.
fn relay_entry_ssh(
    target: &str,
    sender: &str,
    kind: &str,
    body: &str,
    msg_id: &str,
    refs: &[String],
) -> Result<String, String> {
    if target.trim().is_empty() {
        return Err("empty relay target".to_string());
    }
    let argv = relay_argv(target, sender, kind, body, msg_id, refs);
    let output = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|error| format!("ssh to {target} failed to spawn: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("remote append on {target} failed with no message")
        } else {
            format!("remote append on {target} failed: {stderr}")
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
        CoordinationCommand::Send {
            to,
            kind,
            summary,
            body,
            priority,
            in_reply_to,
            r#ref,
            agent,
            log,
            via,
            json,
        } => {
            let log_path = match resolve_log(log.as_deref()) {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("coordination send failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let mut entry =
                CoordinationEntry::new(&agent.unwrap_or_default(), &kind, &body, r#ref, now_unix_s());
            entry.to = Some(to);
            entry.msg_id = Some(uuid::Uuid::new_v4().to_string());
            entry.summary_1line = Some(summary);
            entry.priority = Some(priority);
            entry.in_reply_to = in_reply_to;
            let violations = admit_addressed_append(&entry);
            if !violations.is_empty() {
                eprintln!("coordination send refused: {}", violations.join(", "));
                return ExitCode::FAILURE;
            }
            match append_entry(&log_path, &entry) {
                Ok(()) => {
                    if json {
                        emit_json(&entry, false);
                    } else {
                        println!("sent {} to {}", entry.msg_id.as_deref().unwrap_or("?"), entry.to.as_deref().unwrap_or("?"));
                    }
                    if let Some(target) = via.as_deref() {
                        let msg_id = entry.msg_id.clone().unwrap_or_default();
                        let sender = entry.agent.clone();
                        match relay_entry_ssh(
                            target,
                            &sender,
                            &entry.kind,
                            &entry.body,
                            &msg_id,
                            &entry.refs,
                        ) {
                            Ok(receipt) => {
                                if json {
                                    println!("{receipt}");
                                } else {
                                    println!("relayed {msg_id} via {target}: {receipt}");
                                }
                                ExitCode::SUCCESS
                            }
                            Err(error) => {
                                eprintln!("coordination send relayed locally but remote copy failed: {error}");
                                ExitCode::FAILURE
                            }
                        }
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(error) => {
                    eprintln!("coordination send failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        CoordinationCommand::Inbox {
            agent,
            unopened,
            full,
            limit,
            log,
            json,
        } => {
            let me = agent.unwrap_or_default();
            let log_path = match resolve_log(log.as_deref()) {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("coordination inbox failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let (entries, skipped) = read_entries(&log_path, None, 0);
            let mut opened: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            for entry in &entries {
                if let (Some(id), Some(receipt)) = (&entry.msg_id, &entry.receipt) {
                    if receipt.state == ReceiptState::Opened {
                        opened.insert(id.clone());
                    }
                }
            }
            let mut mine: Vec<&CoordinationEntry> = entries
                .iter()
                .filter(|e| e.receipt.is_none())
                .filter(|e| match &e.to {
                    None => true,
                    Some(to) => {
                        to == &format!("agent:{me}")
                            || to == &me
                            || (*to == "role:all" && e.msg_id.is_some())
                    }
                })
                .filter(|e| {
                    !unopened
                        || e.msg_id.as_ref().is_none_or(|id| !opened.contains(id))
                })
                .collect();
            if limit > 0 && mine.len() > limit {
                mine.drain(..mine.len() - limit);
            }
            if json {
                emit_json(
                    &serde_json::json!({
                        "agent": me,
                        "entries": mine,
                        "skipped_lines": skipped,
                        "log": log_path,
                    }),
                    false,
                );
            } else {
                for entry in &mine {
                    let summary = entry.summary_1line.as_deref().unwrap_or(&entry.body);
                    let id = entry.msg_id.as_deref().unwrap_or("-");
                    let pri = entry.priority.unwrap_or(1);
                    let opened_mark = entry
                        .msg_id
                        .as_ref()
                        .is_some_and(|mid| opened.contains(mid));
                    println!(
                        "{} p{} {} {} [{}]{}",
                        id,
                        pri,
                        entry.agent,
                        entry.kind,
                        summary,
                        if opened_mark { " (opened)" } else { "" }
                    );
                    if full {
                        println!("  {}", entry.body);
                    }
                }
                if skipped > 0 {
                    eprintln!("warning: {skipped} corrupt line(s) skipped");
                }
            }
            ExitCode::SUCCESS
        }
        CoordinationCommand::Ack {
            msg_id,
            state,
            agent,
            log,
            json,
        } => {
            let by = agent.unwrap_or_default();
            let want = match state.to_ascii_lowercase().as_str() {
                "delivered" => ReceiptState::Delivered,
                "opened" => ReceiptState::Opened,
                other => {
                    eprintln!("coordination ack refused: unknown state {other}");
                    return ExitCode::FAILURE;
                }
            };
            let log_path = match resolve_log(log.as_deref()) {
                Ok(path) => path,
                Err(error) => {
                    eprintln!("coordination ack failed: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let (entries, _) = read_entries(&log_path, None, 0);
            if !entries
                .iter()
                .any(|e| e.msg_id.as_deref() == Some(msg_id.as_str()) && e.receipt.is_none())
            {
                eprintln!("coordination ack refused: unknown msg_id {msg_id}");
                return ExitCode::FAILURE;
            }
            let already_opened = entries.iter().any(|e| {
                e.msg_id.as_deref() == Some(msg_id.as_str())
                    && e.receipt.as_ref().is_some_and(|r| r.state == ReceiptState::Opened)
            });
            if already_opened && want == ReceiptState::Delivered {
                eprintln!("coordination ack refused: receipt already opened for {msg_id}");
                return ExitCode::FAILURE;
            }
            let mut entry = CoordinationEntry::new(&by, "info", &format!("ack {msg_id}"), vec![], now_unix_s());
            entry.msg_id = Some(msg_id.clone());
            entry.receipt = Some(MsgReceipt {
                by: by.clone(),
                ts_unix_s: now_unix_s(),
                state: want,
            });
            let violations = admit_receipt(&entry, &by);
            if !violations.is_empty() {
                eprintln!("coordination ack refused: {}", violations.join(", "));
                return ExitCode::FAILURE;
            }
            match append_entry(&log_path, &entry) {
                Ok(()) => {
                    if json {
                        emit_json(&entry, false);
                    } else {
                        println!("acked {msg_id} as {}", state.to_ascii_lowercase());
                    }
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("coordination ack failed: {error}");
                    ExitCode::FAILURE
                }
            }
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
            if let Ok(port) = cynic_vcs::providers::forgejo_pr_port_from_registry(&ws_root) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_survives_hostile_bodies() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("it's $HOME"), "'it'\\''s $HOME'");
        assert_eq!(shell_quote("$(rm -rf /)"), "'$(rm -rf /)'");
    }

    #[test]
    fn relay_argv_links_back_to_local_proof() {
        let argv = relay_argv(
            "cynic-agent-host",
            "hermes",
            "info",
            "hello world",
            "orig-123",
            &["#260".to_string()],
        );
        assert_eq!(
            &argv[..6],
            &["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10", "cynic-agent-host"]
        );
        assert!(argv.contains(&"append".to_string()));
        assert!(argv.contains(&"'hello world'".to_string()));
        let refs: Vec<_> = argv
            .windows(2)
            .filter(|w| w[0] == "--ref")
            .map(|w| w[1].clone())
            .collect();
        assert_eq!(refs, vec!["'orig-123'", "'#260'"]);
        assert_eq!(argv.last().map(String::as_str), Some("--json"));
    }

    #[test]
    fn relay_refuses_empty_target() {
        assert!(relay_entry_ssh("  ", "hermes", "info", "x", "m", &[]).is_err());
    }
}
