//! Fire-and-forget agent coordination log.
//!
//! Agents append debt sightings, blocks, and decisions as single JSON lines
//! without waiting for anyone; readers converge by reading. No locks, no
//! kernel, no network: `O_APPEND` writes are atomic per line, and a missing
//! log reads as empty history. Corrupt lines are skipped loudly so one bad
//! line cannot hide the rest. Cross-machine sharing is a mounted path away
//! (`CYNIC_COORDINATION_LOG`); until then each machine owns its own log and
//! says so.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};

use serde::{Deserialize, Serialize};

pub const COORDINATION_ENTRY_SCHEMA: &str = "cynic.coordination_entry.v1";

/// One fire-and-forget entry. `kind` is a free string by design (`debt`,
/// `info`, `block`, `decision` are conventions, not an enum) so new kinds
/// emerge without a schema migration.
///
/// Addressed messaging rides the same log: `to` + `msg_id` turn an entry
/// into a message, and the receiver answers with a receipt ENTRY (a new
/// line carrying `receipt`, never an in-place edit). All envelope fields
/// are `#[serde(default)]`: old lines read unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationEntry {
    pub schema: String,
    pub ts_unix_s: u64,
    pub agent: String,
    pub kind: String,
    pub body: String,
    #[serde(default)]
    pub refs: Vec<String>,
    /// Addressee: `agent:<id>` or `role:<role>`. Absent = broadcast.
    #[serde(default)]
    pub to: Option<String>,
    /// Sender-generated message id. Required when `to` is set.
    #[serde(default)]
    pub msg_id: Option<String>,
    /// Causal thread pointer (the `msg_id` this answers).
    #[serde(default)]
    pub in_reply_to: Option<String>,
    /// Triage priority 0-3 (default 1 when addressed, unused for broadcast).
    #[serde(default)]
    pub priority: Option<u8>,
    /// One-line triage summary (<= 140 chars). Required when `to` is set.
    #[serde(default)]
    pub summary_1line: Option<String>,
    /// Delivery receipt. Written ONLY by the receiver on a follow-up line;
    /// a sender-set receipt is refused by `admit_addressed_append`.
    #[serde(default)]
    pub receipt: Option<MsgReceipt>,
}

/// Delivery receipt carried by a follow-up log line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MsgReceipt {
    pub by: String,
    pub ts_unix_s: u64,
    pub state: ReceiptState,
}

/// Monotonic receipt state: `delivered` -> `opened`, never backward.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReceiptState {
    Delivered,
    Opened,
}

impl CoordinationEntry {
    pub fn new(agent: &str, kind: &str, body: &str, refs: Vec<String>, now_unix_s: u64) -> Self {
        Self {
            schema: COORDINATION_ENTRY_SCHEMA.to_string(),
            ts_unix_s: now_unix_s,
            agent: agent.to_string(),
            kind: kind.to_string(),
            body: body.to_string(),
            refs,
            to: None,
            msg_id: None,
            in_reply_to: None,
            priority: None,
            summary_1line: None,
            receipt: None,
        }
    }
}

/// Pure admission for an addressed append. Empty = admit.
/// Rules: `to` requires `msg_id` + `summary_1line`; receipt must be absent
/// (receipts arrive as separate lines from the receiver, never from the
/// sender); priority above 3 is refused.
pub fn admit_addressed_append(entry: &CoordinationEntry) -> Vec<String> {
    let mut violations = Vec::new();
    if entry.to.as_ref().is_some_and(|t| !t.trim().is_empty()) {
        if entry.msg_id.as_ref().is_none_or(|m| m.trim().is_empty()) {
            violations.push("addressed_append_msg_id_missing".to_string());
        }
        if entry
            .summary_1line
            .as_ref()
            .is_none_or(|s| s.trim().is_empty())
        {
            violations.push("addressed_append_summary_missing".to_string());
        }
        if entry
            .summary_1line
            .as_ref()
            .is_some_and(|s| s.chars().count() > 140)
        {
            violations.push("addressed_append_summary_too_long".to_string());
        }
        if entry.priority.is_some_and(|p| p > 3) {
            violations.push("addressed_append_priority_out_of_range".to_string());
        }
    }
    if entry.receipt.is_some() {
        violations.push("addressed_append_sender_receipt_forbidden".to_string());
    }
    violations
}

/// Pure admission for a receipt transition. Empty = admit.
/// The receipt author must be the addressee (or any agent for broadcast
/// follow-ups); state must advance monotonically.
pub fn admit_receipt(entry: &CoordinationEntry, by: &str) -> Vec<String> {
    let mut violations = Vec::new();
    match &entry.receipt {
        None => violations.push("receipt_state_missing".to_string()),
        Some(receipt) => {
            if receipt.by.trim().is_empty() || by.trim().is_empty() {
                violations.push("receipt_author_missing".to_string());
            } else if receipt.by != by {
                violations.push("receipt_author_mismatch".to_string());
            }
            if entry.msg_id.as_ref().is_none_or(|m| m.trim().is_empty()) {
                violations.push("receipt_msg_id_missing".to_string());
            }
        }
    }
    violations
}

/// Resolve the log path: explicit `CYNIC_COORDINATION_LOG`, else the
/// operator-local share dir. Never hardcodes a host.
pub fn default_coordination_log_path() -> Result<String, String> {
    if let Ok(explicit) = std::env::var("CYNIC_COORDINATION_LOG") {
        if !explicit.trim().is_empty() {
            return Ok(explicit);
        }
    }
    std::env::var("HOME")
        .map(|home| format!("{home}/.local/share/cynic/coordination.jsonl"))
        .map_err(|_| {
            "coordination log path unresolvable: HOME is unset and CYNIC_COORDINATION_LOG is empty".to_string()
        })
}

/// Append one entry as a single line. Fire-and-forget: no locking, no
/// read-modify-write. Refuses empty bodies and non-entries.
pub fn append_entry(log_path: &str, entry: &CoordinationEntry) -> Result<(), String> {
    if entry.body.trim().is_empty() {
        return Err("refusing to append an entry with an empty body".to_string());
    }
    if entry.agent.trim().is_empty() {
        return Err("refusing to append an entry with an empty agent".to_string());
    }
    if entry.kind.trim().is_empty() {
        return Err("refusing to append an entry with an empty kind".to_string());
    }
    let line = serde_json::to_string(entry)
        .map_err(|error| format!("refusing to append an unserializable entry: {error}"))?;
    if let Some(parent) = std::path::Path::new(log_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!("cannot create coordination log dir {}: {error}", parent.display())
            })?;
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|error| format!("cannot open coordination log {log_path}: {error}"))?;
    writeln!(file, "{line}")
        .map_err(|error| format!("cannot write coordination log {log_path}: {error}"))?;
    Ok(())
}

/// Read entries, oldest first. A missing file reads as empty history.
/// Corrupt lines are counted and reported via `skipped_lines`, never fatal.
pub fn read_entries(
    log_path: &str,
    kind_filter: Option<&str>,
    limit: usize,
) -> (Vec<CoordinationEntry>, usize) {
    let file = match std::fs::File::open(log_path) {
        Ok(file) => file,
        Err(_) => return (vec![], 0),
    };
    let mut entries = Vec::new();
    let mut skipped = 0usize;
    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<CoordinationEntry>(&line) {
            Ok(entry) => {
                if kind_filter.is_some_and(|k| entry.kind != k) {
                    continue;
                }
                entries.push(entry);
            }
            Err(_) => skipped += 1,
        }
    }
    if limit > 0 && entries.len() > limit {
        entries.drain(..entries.len() - limit);
    }
    (entries, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_log(name: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "cynic-coordination-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn append_then_read_roundtrips() {
        let log = tmp_log("roundtrip");
        let entry = CoordinationEntry::new(
            "tester",
            "debt",
            "D0 example debt",
            vec!["PR-1".to_string()],
            1_700_000_000,
        );
        append_entry(&log, &entry).expect("append");
        let (entries, skipped) = read_entries(&log, None, 0);
        assert_eq!(skipped, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], entry);
        let _ = std::fs::remove_file(&log);
    }

    #[test]
    fn missing_log_reads_as_empty_history() {
        let (entries, skipped) = read_entries("/nonexistent/cynic-coord.jsonl", None, 0);
        assert!(entries.is_empty());
        assert_eq!(skipped, 0);
    }

    #[test]
    fn corrupt_lines_are_skipped_loudly() {
        let log = tmp_log("corrupt");
        std::fs::write(&log, "not json\n{\"also\": \"not an entry\"}\n").expect("seed");
        append_entry(
            &log,
            &CoordinationEntry::new("tester", "info", "good line", vec![], 1),
        )
        .expect("append");
        let (entries, skipped) = read_entries(&log, None, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(skipped, 2);
        let _ = std::fs::remove_file(&log);
    }

    #[test]
    fn kind_filter_and_limit_hold() {
        let log = tmp_log("filter");
        for (i, kind) in ["debt", "info", "debt"].iter().enumerate() {
            append_entry(
                &log,
                &CoordinationEntry::new("tester", kind, &format!("body {i}"), vec![], i as u64),
            )
            .expect("append");
        }
        let (debts, _) = read_entries(&log, Some("debt"), 0);
        assert_eq!(debts.len(), 2);
        let (last_one, _) = read_entries(&log, None, 1);
        assert_eq!(last_one.len(), 1);
        assert_eq!(last_one[0].body, "body 2");
        let _ = std::fs::remove_file(&log);
    }

    #[test]
    fn empty_body_agent_or_kind_is_refused() {
        let log = tmp_log("refused");
        for (agent, kind, body) in [("", "debt", "x"), ("a", "", "x"), ("a", "debt", "  ")] {
            let entry = CoordinationEntry::new(agent, kind, body, vec![], 1);
            assert!(append_entry(&log, &entry).is_err());
        }
        assert!(!std::path::Path::new(&log).exists());
    }

    fn addressed(sender: &str, to: &str) -> CoordinationEntry {
        let mut entry = CoordinationEntry::new(sender, "info", "full body here", vec![], 2);
        entry.to = Some(to.to_string());
        entry.msg_id = Some("msg-1".to_string());
        entry.summary_1line = Some("one-line triage".to_string());
        entry.priority = Some(1);
        entry
    }

    #[test]
    fn addressed_append_admits_complete_envelope() {
        assert!(admit_addressed_append(&addressed("a", "agent:b")).is_empty());
    }

    #[test]
    fn addressed_append_requires_msg_id_and_summary() {
        let mut no_id = addressed("a", "agent:b");
        no_id.msg_id = None;
        assert!(admit_addressed_append(&no_id)
            .contains(&"addressed_append_msg_id_missing".to_string()));
        let mut no_summary = addressed("a", "agent:b");
        no_summary.summary_1line = Some("   ".to_string());
        assert!(admit_addressed_append(&no_summary)
            .contains(&"addressed_append_summary_missing".to_string()));
        let mut long_summary = addressed("a", "agent:b");
        long_summary.summary_1line = Some("x".repeat(141));
        assert!(admit_addressed_append(&long_summary)
            .contains(&"addressed_append_summary_too_long".to_string()));
        let mut bad_priority = addressed("a", "agent:b");
        bad_priority.priority = Some(9);
        assert!(admit_addressed_append(&bad_priority)
            .contains(&"addressed_append_priority_out_of_range".to_string()));
    }

    #[test]
    fn sender_set_receipt_is_refused() {
        let mut entry = addressed("a", "agent:b");
        entry.receipt = Some(MsgReceipt {
            by: "a".to_string(),
            ts_unix_s: 3,
            state: ReceiptState::Delivered,
        });
        assert!(admit_addressed_append(&entry)
            .contains(&"addressed_append_sender_receipt_forbidden".to_string()));
    }

    #[test]
    fn receipt_admission_checks_author_and_msg_id() {
        let mut entry = CoordinationEntry::new("b", "info", "ack", vec![], 3);
        entry.msg_id = Some("msg-1".to_string());
        entry.receipt = Some(MsgReceipt {
            by: "b".to_string(),
            ts_unix_s: 3,
            state: ReceiptState::Opened,
        });
        assert!(admit_receipt(&entry, "b").is_empty());
        assert!(admit_receipt(&entry, "c")
            .contains(&"receipt_author_mismatch".to_string()));
        let bare = CoordinationEntry::new("b", "info", "ack", vec![], 3);
        assert!(admit_receipt(&bare, "b")
            .contains(&"receipt_state_missing".to_string()));
    }

    #[test]
    fn old_lines_without_envelope_still_read() {
        let log = tmp_log("backward");
        std::fs::write(
            &log,
            "{\"schema\":\"cynic.coordination_entry.v1\",\"ts_unix_s\":1,\"agent\":\"a\",\"kind\":\"debt\",\"body\":\"old\",\"refs\":[]}\n",
        )
        .expect("seed");
        let (entries, skipped) = read_entries(&log, None, 0);
        assert_eq!(skipped, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].to, None);
        assert_eq!(entries[0].receipt, None);
        let _ = std::fs::remove_file(&log);
    }
}
